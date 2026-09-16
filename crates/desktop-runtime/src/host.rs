//! 可嵌入宿主的执行线程；Tauri 与 CLI 共享连接、取消与表现清理逻辑。

use crate::{
    audio::{DeviceBackend, SimulatedBackend},
    config::ClientConfig,
    connection::run_once_with_mouth,
    presentation::AvatarDriver,
};
use std::{
    future::Future,
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
};
use tokio::{
    sync::oneshot,
    time::{Duration, sleep},
};

#[derive(Clone, Debug, serde::Serialize)]
pub struct RuntimeStatus {
    pub running: bool,
    pub simulation: bool,
    pub last_error: Option<String>,
}

pub struct RuntimeHandle {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
    status: Arc<Mutex<RuntimeStatus>>,
}

impl RuntimeHandle {
    pub fn start(config: ClientConfig, simulation: bool) -> Result<Self, String> {
        let (stop, stopped) = oneshot::channel();
        let (ready, initialized) = mpsc::sync_channel(1);
        let status = Arc::new(Mutex::new(RuntimeStatus {
            running: false,
            simulation,
            last_error: None,
        }));
        let worker_status = status.clone();
        let thread = std::thread::Builder::new()
            .name("meowlive-execution".into())
            .spawn(move || {
                let prepared = (|| {
                    if !simulation {
                        let _ = DeviceBackend::new(config.max_buffer_samples)?;
                    }
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|error| error.to_string())
                })();
                let runtime = match prepared {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let driver = {
                    let _guard = runtime.enter();
                    AvatarDriver::start(config.vtube_studio.clone())
                };
                let driver = match driver {
                    Ok(driver) => driver,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                worker_status
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .running = true;
                let _ = ready.send(Ok(()));
                let result = runtime.block_on(run_client_with_driver(
                    config,
                    simulation,
                    false,
                    async {
                        let _ = stopped.await;
                        Ok(())
                    },
                    driver,
                ));
                runtime.shutdown_timeout(Duration::from_millis(250));
                let mut state = worker_status.lock().unwrap_or_else(|e| e.into_inner());
                state.running = false;
                state.last_error = result.err();
            })
            .map_err(|error| error.to_string())?;
        match initialized
            .recv()
            .map_err(|_| "desktop runtime initialization terminated".to_owned())?
        {
            Ok(()) => Ok(Self {
                stop: Some(stop),
                thread: Some(thread),
                status,
            }),
            Err(error) => {
                let _ = thread.join();
                Err(error)
            }
        }
    }

    pub fn status(&self) -> RuntimeStatus {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| "desktop execution thread panicked".to_owned())?;
        }
        Ok(())
    }
}

impl Drop for RuntimeHandle {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

pub(crate) async fn run_client(
    config: ClientConfig,
    simulation: bool,
    once: bool,
    shutdown: impl Future<Output = Result<(), String>>,
) -> Result<(), String> {
    let driver = AvatarDriver::start(config.vtube_studio.clone())?;
    run_client_with_driver(config, simulation, once, shutdown, driver).await
}

async fn run_client_with_driver(
    config: ClientConfig,
    simulation: bool,
    once: bool,
    shutdown: impl Future<Output = Result<(), String>>,
    driver: AvatarDriver,
) -> Result<(), String> {
    let mut status = driver.status();
    let status_log = tokio::spawn(async move {
        while status.changed().await.is_ok() {
            eprintln!("VTube Studio: {:?}", *status.borrow_and_update());
        }
    });
    tokio::pin!(shutdown);
    let result = async {
        loop {
            let session = async {
                if simulation {
                    let backend = driver.observe(
                        SimulatedBackend::new(config.max_buffer_samples),
                        config.lip_sync.clone(),
                    )?;
                    run_once_with_mouth(&config, backend, Some(driver.mouth_parameter())).await
                } else {
                    let backend = driver.observe(
                        DeviceBackend::new(config.max_buffer_samples)?,
                        config.lip_sync.clone(),
                    )?;
                    run_once_with_mouth(&config, backend, Some(driver.mouth_parameter())).await
                }
            };
            let result = tokio::select! {
                biased;
                result = &mut shutdown => return result,
                result = session => result,
            };
            if once {
                return result;
            }
            if let Err(error) = result {
                eprintln!(
                    "desktop connection ended: {error}; reconnecting after {} ms",
                    config.reconnect_delay_ms
                );
            }
            tokio::select! {
                result = &mut shutdown => return result,
                _ = sleep(Duration::from_millis(config.reconnect_delay_ms)) => {},
            }
        }
    }
    .await;
    driver.shutdown().await;
    status_log.abort();
    let _ = status_log.await;
    result
}

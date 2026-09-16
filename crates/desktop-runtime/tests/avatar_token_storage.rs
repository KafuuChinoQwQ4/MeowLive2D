mod avatar_support;
use avatar_support::*;
use meowlive_desktop_runtime::avatar::AvatarStatus;

#[tokio::test]
async fn malformed_or_oversized_cached_tokens_fail_visibly_without_network() {
    for token in ["".to_string(), "x".repeat(65), "私有令牌".into()] {
        let fixture = Fixture::new(false).await;
        write_token(&fixture.config.token_path, &token);
        let (_levels, status, task) = fixture.start();
        finished(task).await;
        assert!(matches!(*status.borrow(), AvatarStatus::Failed(_)));
        assert!(
            !format!("{:?}", status.borrow())
                .contains(&fixture.config.token_path.to_string_lossy().to_string())
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn symbolic_link_token_is_not_read_or_overwritten() {
    let fixture = Fixture::new(false).await;
    let other = fixture.config.token_path.with_extension("outside");
    std::fs::write(&other, "untouched").unwrap();
    std::os::unix::fs::symlink(&other, &fixture.config.token_path).unwrap();
    let (_levels, status, task) = fixture.start();
    finished(task).await;
    assert!(matches!(*status.borrow(), AvatarStatus::Failed(_)));
    assert_eq!(std::fs::read_to_string(other).unwrap(), "untouched");
}

#[test]
fn shutdown_cancels_queued_token_io_without_waiting_for_the_disk_worker() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let (release, wait) = std::sync::mpsc::channel();
        let (started, ready) = tokio::sync::oneshot::channel();
        let blocked_disk = tokio::task::spawn_blocking(move || {
            started.send(()).unwrap();
            wait.recv().unwrap();
        });
        ready.await.unwrap();
        let fixture = Fixture::new(false).await;
        std::fs::write(&fixture.config.token_path, "invalid json").unwrap();
        let (levels, status, task) = fixture.start();
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        drop(levels);
        let result = tokio::time::timeout(std::time::Duration::from_millis(100), task).await;
        let final_status = status.borrow().clone();
        release.send(()).unwrap();
        blocked_disk.await.unwrap();
        result
            .expect("a pending disk read must not hold the audio owner open")
            .unwrap();
        assert_eq!(
            final_status,
            AvatarStatus::Disconnected,
            "cancel pending disk work before reporting an unrelated file error"
        );
    });
}

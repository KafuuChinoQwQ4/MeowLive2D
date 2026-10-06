//! 持久化分句并行数；保存成功后原子更新，后续合成请求读取同一份设置。
use meowlive_protocol::control::SpeechSettings;
use std::{
    io::Write,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

pub struct SpeechSettingsStore {
    pub batch_size: Arc<AtomicU8>,
    path: Option<PathBuf>,
    edits: Mutex<()>,
}
impl Default for SpeechSettingsStore {
    fn default() -> Self {
        Self {
            batch_size: Arc::new(AtomicU8::new(4)),
            path: None,
            edits: Mutex::new(()),
        }
    }
}
impl SpeechSettingsStore {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        let mut store = Self::default();
        match std::fs::symlink_metadata(&path) {
            Ok(meta) => {
                if !meta.is_file() || meta.len() > 1024 {
                    return Err("语音合成设置文件无效".into());
                }
                let bytes = std::fs::read(&path).map_err(|_| "无法读取语音合成设置")?;
                let settings: SpeechSettings =
                    serde_json::from_slice(&bytes).map_err(|_| "语音合成设置格式无效")?;
                validate(settings.sentence_batch_size)?;
                store
                    .batch_size
                    .store(settings.sentence_batch_size, Ordering::Relaxed);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("无法读取语音合成设置".into()),
        }
        store.path = Some(path);
        Ok(store)
    }
    pub fn snapshot(&self) -> SpeechSettings {
        SpeechSettings {
            sentence_batch_size: self.batch_size.load(Ordering::Relaxed),
        }
    }
    pub fn save(&self, settings: SpeechSettings) -> Result<SpeechSettings, String> {
        validate(settings.sentence_batch_size)?;
        let _guard = self.edits.lock().map_err(|_| "语音合成设置暂不可用")?;
        if let Some(path) = &self.path {
            let parent = path.parent().ok_or("语音合成设置目录无效")?;
            std::fs::create_dir_all(parent).map_err(|_| "无法建立语音合成设置目录")?;
            let temporary = parent.join(format!(".speech-settings-{}.tmp", uuid::Uuid::new_v4()));
            let result = (|| -> std::io::Result<()> {
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)?;
                file.write_all(&serde_json::to_vec(&settings)?)?;
                file.sync_all()?;
                std::fs::rename(&temporary, path)
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(&temporary);
                return Err("无法保存语音合成设置，原设置保持不变".into());
            }
        }
        self.batch_size
            .store(settings.sentence_batch_size, Ordering::Relaxed);
        Ok(settings)
    }
}
pub fn validate(value: u8) -> Result<(), String> {
    if !(1..=16).contains(&value) {
        return Err("并行合成分句数须为 1～16 的整数".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_survive_reopen_and_failed_writes_keep_previous_value() {
        let dir = std::env::temp_dir().join(format!("speech-settings-{}", uuid::Uuid::new_v4()));
        let path = dir.join("settings.json");
        let store = SpeechSettingsStore::open(path.clone()).unwrap();
        store
            .save(SpeechSettings {
                sentence_batch_size: 16,
            })
            .unwrap();
        assert_eq!(
            SpeechSettingsStore::open(path.clone())
                .unwrap()
                .snapshot()
                .sentence_batch_size,
            16
        );
        std::fs::remove_file(&path).unwrap();
        std::fs::create_dir(&path).unwrap();
        assert!(
            store
                .save(SpeechSettings {
                    sentence_batch_size: 2
                })
                .is_err()
        );
        assert_eq!(store.snapshot().sentence_batch_size, 16);
        std::fs::remove_dir_all(dir).unwrap();
    }
}

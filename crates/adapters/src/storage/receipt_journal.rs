//! 最小回执原子日志；文件锁序列化跨实例更新，错误不丢弃待恢复事实。
use meowlive_application::ports::{
    receipt_journal::{ReceiptJournal, ReceiptRecord},
    viewers::ViewerStoreError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
const MAX: usize = 4096;
const RECORD_BYTES: u64 = 4096;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    scope: String,
    speech_id: String,
    completed_at_ms: u64,
    attempts: u32,
    last_error: Option<String>,
}
pub struct FileReceiptJournal {
    directory: PathBuf,
    capacity: usize,
}
fn error(message: &str) -> ViewerStoreError {
    ViewerStoreError::new(message)
}
fn key(value: &str) -> Result<(), ViewerStoreError> {
    if value.trim().is_empty()
        || value.len() > 128
        || !value.bytes().all(|b| (b' '..=b'~').contains(&b))
    {
        Err(error("invalid receipt journal identity"))
    } else {
        Ok(())
    }
}
fn filename(scope: &str, speech: &str) -> String {
    let bytes = serde_json::to_vec(&(scope, speech)).expect("string tuple");
    format!("{:x}.json", Sha256::digest(bytes))
}
impl FileReceiptJournal {
    pub fn open(directory: &Path, capacity: usize) -> Result<Self, ViewerStoreError> {
        if !(1..=MAX).contains(&capacity) {
            return Err(error("receipt journal capacity must be 1..4096"));
        }
        fs::create_dir_all(directory)
            .map_err(|_| error("cannot create receipt journal directory"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .map_err(|_| error("cannot secure receipt journal directory"))?;
        }
        if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
            File::open(parent)
                .and_then(|f| f.sync_all())
                .map_err(|_| error("receipt journal parent sync failed"))?;
        }
        let this = Self {
            directory: directory.to_path_buf(),
            capacity,
        };
        let _guard = this.lock()?;
        this.cleanup_orphans()?;
        let rows = this.read()?;
        if rows.len() > capacity {
            return Err(error("receipt journal existing records exceed capacity"));
        }
        Ok(this)
    }
    fn cleanup_orphans(&self) -> Result<(), ViewerStoreError> {
        for entry in fs::read_dir(&self.directory)
            .map_err(|_| error("cannot scan receipt journal orphans"))?
        {
            let entry = entry.map_err(|_| error("cannot inspect receipt journal orphan"))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name
                .strip_prefix(".receipt-tmp-")
                .is_some_and(|suffix| uuid::Uuid::parse_str(suffix).is_ok())
                && entry
                    .file_type()
                    .map_err(|_| error("cannot inspect receipt journal orphan"))?
                    .is_file()
            {
                fs::remove_file(entry.path())
                    .map_err(|_| error("cannot clean receipt journal orphan"))?;
            }
        }
        self.sync_dir()
    }
    fn lock(&self) -> Result<File, ViewerStoreError> {
        let file = private_options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.directory.join(".journal.lock"))
            .map_err(|_| error("cannot open receipt journal lock"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))
                .map_err(|_| error("cannot secure receipt journal lock"))?;
        }
        fs2::FileExt::lock_exclusive(&file).map_err(|_| error("cannot lock receipt journal"))?;
        Ok(file)
    }
    fn sync_dir(&self) -> Result<(), ViewerStoreError> {
        File::open(&self.directory)
            .and_then(|f| f.sync_all())
            .map_err(|_| error("receipt journal directory sync failed; durability uncertain"))
    }
    fn read(&self) -> Result<Vec<Stored>, ViewerStoreError> {
        let mut rows = Vec::new();
        for entry in
            fs::read_dir(&self.directory).map_err(|_| error("cannot read receipt journal"))?
        {
            let entry = entry.map_err(|_| error("cannot read receipt journal entry"))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == ".journal.lock" || name.starts_with(".receipt-tmp-") {
                continue;
            }
            if !entry
                .file_type()
                .map_err(|_| error("cannot inspect receipt journal entry"))?
                .is_file()
                || !name.ends_with(".json")
            {
                return Err(error("unexpected receipt journal entry; recovery required"));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(entry.path(), fs::Permissions::from_mode(0o600))
                    .map_err(|_| error("cannot secure receipt journal record"))?;
            }
            let mut bytes = Vec::new();
            File::open(entry.path())
                .and_then(|f| f.take(RECORD_BYTES + 1).read_to_end(&mut bytes))
                .map_err(|_| error("cannot read receipt journal record"))?;
            if bytes.len() > RECORD_BYTES as usize {
                return Err(error("receipt journal record exceeds size bound"));
            }
            let row: Stored = serde_json::from_slice(&bytes)
                .map_err(|_| error("corrupt receipt journal record; recovery required"))?;
            key(&row.scope)?;
            key(&row.speech_id)?;
            if row.completed_at_ms > i64::MAX as u64
                || row.last_error.as_ref().is_some_and(|s| s.len() > 256)
                || name != filename(&row.scope, &row.speech_id)
            {
                return Err(error("receipt journal record identity or bounds mismatch"));
            }
            rows.push(row);
            if rows.len() > MAX {
                return Err(error("receipt journal record count exceeds hard limit"));
            }
        }
        Ok(rows)
    }
    fn write(&self, row: &Stored) -> Result<(), ViewerStoreError> {
        let path = self.directory.join(filename(&row.scope, &row.speech_id));
        let tmp = self
            .directory
            .join(format!(".receipt-tmp-{}", uuid::Uuid::new_v4()));
        let result = (|| {
            let bytes = serde_json::to_vec(row)
                .map_err(|_| error("cannot encode receipt journal record"))?;
            let mut f = private_options()
                .create_new(true)
                .write(true)
                .open(&tmp)
                .map_err(|_| error("cannot create receipt journal temporary record"))?;
            f.write_all(&bytes)
                .and_then(|()| f.sync_all())
                .map_err(|_| error("cannot sync receipt journal record"))?;
            fs::rename(&tmp, &path)
                .map_err(|_| error("cannot atomically replace receipt journal record"))?;
            self.sync_dir()
        })();
        if result.is_err() {
            let _ = fs::remove_file(tmp);
        }
        result
    }
}
impl ReceiptJournal for FileReceiptJournal {
    fn put(
        &self,
        scope: &str,
        speech_id: &str,
        completed_at_ms: u64,
    ) -> Result<(), ViewerStoreError> {
        key(scope)?;
        key(speech_id)?;
        if completed_at_ms > i64::MAX as u64 {
            return Err(error("invalid receipt completion timestamp"));
        }
        let _guard = self.lock()?;
        let rows = self.read()?;
        if rows
            .iter()
            .any(|r| r.scope == scope && r.speech_id == speech_id)
        {
            return Ok(());
        }
        if rows.len() >= self.capacity {
            return Err(error("receipt journal capacity exceeded"));
        }
        self.write(&Stored {
            scope: scope.into(),
            speech_id: speech_id.into(),
            completed_at_ms,
            attempts: 0,
            last_error: None,
        })
    }
    fn pending(&self, scope: &str, limit: usize) -> Result<Vec<ReceiptRecord>, ViewerStoreError> {
        key(scope)?;
        if !(1..=MAX).contains(&limit) {
            return Err(error("invalid receipt journal page limit"));
        }
        let _guard = self.lock()?;
        let mut rows: Vec<_> = self
            .read()?
            .into_iter()
            .filter(|r| r.scope == scope)
            .collect();
        rows.sort_by(|a, b| {
            (a.attempts, a.completed_at_ms, &a.speech_id).cmp(&(
                b.attempts,
                b.completed_at_ms,
                &b.speech_id,
            ))
        });
        Ok(rows
            .into_iter()
            .take(limit)
            .map(|r| ReceiptRecord {
                speech_id: r.speech_id,
                completed_at_ms: r.completed_at_ms,
                attempts: r.attempts,
                last_error: r.last_error,
            })
            .collect())
    }
    fn acknowledge(&self, scope: &str, speech_id: &str) -> Result<(), ViewerStoreError> {
        key(scope)?;
        key(speech_id)?;
        let _guard = self.lock()?;
        self.read()?;
        match fs::remove_file(self.directory.join(filename(scope, speech_id))) {
            Ok(()) => self.sync_dir(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(error("cannot acknowledge receipt journal record")),
        }
    }
    fn failed(&self, scope: &str, speech_id: &str) -> Result<(), ViewerStoreError> {
        key(scope)?;
        key(speech_id)?;
        let _guard = self.lock()?;
        let mut row = self
            .read()?
            .into_iter()
            .find(|r| r.scope == scope && r.speech_id == speech_id)
            .ok_or_else(|| error("receipt journal record missing"))?;
        row.attempts = row.attempts.saturating_add(1);
        row.last_error = Some("completed receipt database submission failed".into());
        self.write(&row)
    }
}

fn private_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}

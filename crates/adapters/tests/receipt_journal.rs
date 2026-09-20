use meowlive_adapters::storage::receipt_journal::FileReceiptJournal;
use meowlive_application::ports::receipt_journal::ReceiptJournal;
fn dir() -> std::path::PathBuf {
    std::env::temp_dir().join(format!("receipt-test-{}", uuid::Uuid::new_v4()))
}
#[test]
fn restart_duplicate_capacity_and_scope_are_durable() {
    let d = dir();
    let j = FileReceiptJournal::open(&d, 2).unwrap();
    j.put("scope", "speech", 100).unwrap();
    j.put("scope", "speech", 999).unwrap();
    j.put("other", "speech", 200).unwrap();
    assert!(j.put("scope", "full", 300).is_err());
    drop(j);
    let j = FileReceiptJournal::open(&d, 2).unwrap();
    assert_eq!(j.pending("scope", 10).unwrap()[0].completed_at_ms, 100);
    assert_eq!(j.pending("other", 10).unwrap()[0].completed_at_ms, 200);
    j.acknowledge("scope", "speech").unwrap();
    j.put("scope", "next", 300).unwrap();
    drop(j);
    assert_eq!(
        FileReceiptJournal::open(&d, 2)
            .unwrap()
            .pending("scope", 10)
            .unwrap()[0]
            .speech_id,
        "next"
    );
    std::fs::remove_dir_all(d).unwrap();
}
#[test]
fn poison_receipt_moves_behind_unattempted_receipts() {
    let d = dir();
    let j = FileReceiptJournal::open(&d, 4).unwrap();
    j.put("s", "old", 1).unwrap();
    j.put("s", "new", 2).unwrap();
    j.failed("s", "old").unwrap();
    let p = j.pending("s", 4).unwrap();
    assert_eq!(p[0].speech_id, "new");
    assert_eq!(p[1].attempts, 1);
    assert!(p[1].last_error.is_some());
    std::fs::remove_dir_all(d).unwrap();
}
#[test]
fn corrupt_record_is_reported_and_not_discarded() {
    let d = dir();
    let j = FileReceiptJournal::open(&d, 4).unwrap();
    j.put("s", "id", 1).unwrap();
    let file = std::fs::read_dir(&d)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "json"))
        .unwrap();
    std::fs::write(&file, b"broken").unwrap();
    assert!(j.pending("s", 4).is_err());
    assert!(FileReceiptJournal::open(&d, 4).is_err());
    assert_eq!(std::fs::read(file).unwrap(), b"broken");
    std::fs::remove_dir_all(d).unwrap();
}
#[test]
fn failed_replace_keeps_prior_receipt_and_does_not_lose_completion() {
    let d = dir();
    let j = FileReceiptJournal::open(&d, 4).unwrap();
    j.put("s", "old", 7).unwrap();
    // A foreign filesystem entry makes recovery fail closed before any mutation.
    std::fs::create_dir(d.join("unexpected")).unwrap();
    assert!(j.failed("s", "old").is_err());
    std::fs::remove_dir(d.join("unexpected")).unwrap();
    let old = j.pending("s", 4).unwrap();
    assert_eq!(old[0].completed_at_ms, 7);
    assert_eq!(old[0].attempts, 0);
    drop(j);
    assert_eq!(
        FileReceiptJournal::open(&d, 4)
            .unwrap()
            .pending("s", 4)
            .unwrap()[0],
        old[0]
    );
    std::fs::remove_dir_all(d).unwrap();
}
#[test]
fn multiple_instances_serialize_duplicates_and_capacity() {
    let d = dir();
    let a = FileReceiptJournal::open(&d, 1).unwrap();
    let b = FileReceiptJournal::open(&d, 1).unwrap();
    std::thread::scope(|s| {
        let first = s.spawn(|| a.put("s", "same", 1));
        let second = s.spawn(|| b.put("s", "same", 2));
        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();
    });
    assert_eq!(a.pending("s", 4).unwrap().len(), 1);
    assert!(b.put("s", "other", 3).is_err());
    std::fs::remove_dir_all(d).unwrap();
}
#[cfg(unix)]
#[test]
fn private_permissions_and_valid_orphan_cleanup() {
    use std::os::unix::fs::PermissionsExt;
    let d = dir();
    let j = FileReceiptJournal::open(&d, 4).unwrap();
    j.put("s", "id", 1).unwrap();
    assert_eq!(
        std::fs::metadata(&d).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for e in std::fs::read_dir(&d).unwrap() {
        assert_eq!(
            e.unwrap().metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let orphan = d.join(format!(".receipt-tmp-{}", uuid::Uuid::new_v4()));
    std::fs::write(&orphan, b"partial").unwrap();
    drop(j);
    FileReceiptJournal::open(&d, 4).unwrap();
    assert!(!orphan.exists());
    std::fs::remove_dir_all(d).unwrap();
}

use super::*;
use ed25519_dalek::{Signer, SigningKey};
#[test]
fn chunks_use_separate_release_with_legacy_fallback() {
    let urls = chunk_urls("v0.1.2", None, "abc").unwrap();
    assert_eq!(
        urls,
        [
            "https://github.com/KafuuChinoQwQ4/MeowLive2D/releases/download/v0.1.2-updates-windows-x86_64/chunk-abc.bin",
            "https://github.com/KafuuChinoQwQ4/MeowLive2D/releases/download/v0.1.2/chunk-abc.bin",
        ]
    );
    assert_eq!(version("v0.1.2-updates-windows-x86_64"), None);
    assert_eq!(
        version("v0.1.2-windows-preview.20261004-updates-windows-x86_64"),
        None
    );
}

#[test]
fn linux_updates_use_linux_chunk_release_and_package_bound_installers() {
    let urls = chunk_urls("v0.1.2", Some("linux-appimage-x86_64"), "abc").unwrap();
    assert!(urls[0].contains("v0.1.2-updates-linux-x86_64"));
    assert!(installer_matches_target(
        "MeowLive2D.AppImage",
        Some("linux-appimage-x86_64")
    ));
    assert!(installer_matches_target(
        "meowlive.deb",
        Some("linux-deb-x86_64")
    ));
    assert!(!installer_matches_target(
        "MeowLive2D_setup.exe",
        Some("linux-appimage-x86_64")
    ));
    assert!(!installer_matches_target(
        "MeowLive2D.AppImage",
        Some("linux-deb-x86_64")
    ));
}
#[test]
fn dated_previews_and_stable_versions_are_ordered() {
    assert!(
        version("v0.1.1-windows-preview.20261004") > version("v0.1.1-windows-preview.20260929")
    );
    assert!(version("v0.1.1") > version("v0.1.1-windows-preview.20261004"));
    assert!(version("v0.2.0-windows-preview.20260101") > version("v0.1.1"));
    assert_eq!(version("../../bad"), None);
}
#[test]
fn signature_binds_manifest_and_release_tag() {
    let key = SigningKey::from_bytes(&[42; 32]);
    let bytes = b"installer";
    let manifest = Manifest {
        schema: 1,
        tag: "v0.1.1".into(),
        target: Some("windows-x64".into()),
        installer: "app-setup.exe".into(),
        size: bytes.len() as u64,
        sha256: digest(bytes),
        chunks: vec![Chunk {
            sha256: digest(bytes),
            size: bytes.len() as u64,
        }],
    };
    let payload = serde_json::to_vec(&manifest).unwrap();
    let mut envelope = Envelope {
        payload: BASE64.encode(&payload),
        signature: BASE64.encode(key.sign(&payload).to_bytes()),
    };
    let public = BASE64.encode(key.verifying_key().to_bytes());
    assert!(verify_manifest(&envelope, &public, "v0.1.1").is_ok());
    assert!(verify_manifest(&envelope, &public, "v0.1.2").is_err());
    envelope.payload = BASE64.encode(b"tampered");
    assert!(verify_manifest(&envelope, &public, "v0.1.1").is_err());
}
#[test]
fn verified_cache_reuses_only_unchanged_bytes() {
    let bytes = b"unchanged";
    let block = Chunk {
        sha256: digest(bytes),
        size: bytes.len() as u64,
    };
    assert!(valid_chunk(bytes, &block));
    assert!(!valid_chunk(b"corrupted", &block));
    assert!(!valid_chunk(b"short", &block));
}
#[test]
fn delta_reconstruction_downloads_missing_and_repairs_corrupt_cache() {
    let directory =
        std::env::temp_dir().join(format!("meowlive-update-test-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let previous = vec![7; BLOCK_SIZE as usize];
    let changed = b"new tail".to_vec();
    let first = Chunk {
        sha256: digest(&previous),
        size: previous.len() as u64,
    };
    let second = Chunk {
        sha256: digest(&changed),
        size: changed.len() as u64,
    };
    let first_path = directory.join(&first.sha256);
    let second_path = directory.join(&second.sha256);
    fs::write(&first_path, &previous).unwrap();
    fs::write(&second_path, b"corrupt!").unwrap();
    let mut requests = 0;
    let (reused, hit) = load_chunk(&first_path, &first, || {
        requests += 1;
        Err("unexpected download".into())
    })
    .unwrap();
    assert!(hit);
    let (downloaded, hit) = load_chunk(&second_path, &second, || {
        requests += 1;
        Ok(changed.clone())
    })
    .unwrap();
    assert!(!hit);
    assert_eq!(requests, 1);
    assert_eq!(fs::read(&second_path).unwrap(), changed);
    assert_eq!(
        digest(&[reused, downloaded].concat()),
        digest(&[previous, changed].concat())
    );
    assert!(
        load_chunk(&directory.join("missing"), &second, || Ok(
            b"tampered".to_vec()
        ))
        .is_err()
    );
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn redirect_allowlist_rejects_foreign_and_plaintext_hosts() {
    for url in [
        "http://github.com/a",
        "https://github.com.evil.test/a",
        "https://example.com/a",
        "https://github.com:444/a",
        "https://user@github.com/a",
    ] {
        assert!(!trusted_url(&url::Url::parse(url).unwrap()));
    }
    assert!(trusted_url(
        &url::Url::parse("https://release-assets.githubusercontent.com/asset").unwrap()
    ));
}
#[test]
fn installation_rechecks_file_even_after_download() {
    let directory =
        std::env::temp_dir().join(format!("meowlive-install-test-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("installer.exe");
    let bytes = b"verified installer";
    let manifest = Manifest {
        schema: 1,
        tag: "v0.1.1".into(),
        target: Some("windows-x64".into()),
        installer: "app-setup.exe".into(),
        size: bytes.len() as u64,
        sha256: digest(bytes),
        chunks: vec![],
    };
    fs::write(&path, bytes).unwrap();
    assert!(verify_file(&path, &manifest).is_ok());
    fs::write(&path, b"tampered installer").unwrap();
    assert!(verify_file(&path, &manifest).is_err());
    fs::remove_dir_all(directory).unwrap();
}
#[test]
fn signed_manifest_accepts_content_defined_boundaries() {
    let key = SigningKey::from_bytes(&[41; 32]);
    let public = BASE64.encode(key.verifying_key().to_bytes());
    let first = vec![3; 300_000];
    let last = b"tail";
    let manifest = Manifest {
        schema: 1,
        tag: "v0.1.2".into(),
        target: Some("windows-x64".into()),
        installer: "app-setup.exe".into(),
        size: (first.len() + last.len()) as u64,
        sha256: digest(&[first.as_slice(), last].concat()),
        chunks: vec![
            Chunk {
                sha256: digest(&first),
                size: first.len() as u64,
            },
            Chunk {
                sha256: digest(last),
                size: last.len() as u64,
            },
        ],
    };
    let payload = serde_json::to_vec(&manifest).unwrap();
    let envelope = Envelope {
        payload: BASE64.encode(&payload),
        signature: BASE64.encode(key.sign(&payload).to_bytes()),
    };
    assert!(verify_manifest(&envelope, &public, "v0.1.2").is_ok());
}

#[test]
fn plain_release_tag_is_recognized_without_accepting_update_resource_tags() {
    assert_eq!(version("0.1.3"), Some((0, 1, 3, u64::MAX)));
    assert_eq!(version("0.1.3-updates-linux-x86_64"), None);
}

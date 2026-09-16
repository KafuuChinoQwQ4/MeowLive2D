use meowlive_adapters::live::bilibili::signing::sign;

#[test]
fn signs_the_official_canonical_header_order() {
    let signed = sign(
        "test-key",
        "test-secret",
        br#"{"code":"identity","app_id":42}"#,
        1_700_000_000,
        "fixed-nonce",
    )
    .unwrap();

    assert_eq!(signed.content_md5, "2a152c90fd58d83be256b707b63ddac5");
    assert_eq!(
        signed.authorization,
        "229dca415101d976530fe7f56e2b11205f4a55e1f95dcc93ba755e6c2ec642f0"
    );
    assert_eq!(
        signed.canonical,
        "x-bili-accesskeyid:test-key\n\
x-bili-content-md5:2a152c90fd58d83be256b707b63ddac5\n\
x-bili-signature-method:HMAC-SHA256\n\
x-bili-signature-nonce:fixed-nonce\n\
x-bili-signature-version:1.0\n\
x-bili-timestamp:1700000000"
    );
}

#[test]
fn rejects_header_injection_in_credentials_and_nonce() {
    for (key, nonce) in [("bad\nkey", "nonce"), ("key", "bad\rnonce")] {
        assert!(sign(key, "secret", b"{}", 1, nonce).is_err());
    }
}

//! 直播开放平台请求签名，输入固定时间与 nonce 以便调用方独立测试。

use hmac::{Hmac, Mac};
use md5::{Digest as _, Md5};
use sha2::Sha256;

#[derive(Clone, Eq, PartialEq)]
pub struct SignedHeaders {
    pub content_md5: String,
    pub timestamp: String,
    pub nonce: String,
    pub access_key_id: String,
    pub canonical: String,
    pub authorization: String,
}

pub fn sign(
    access_key_id: &str,
    access_key_secret: &str,
    body: &[u8],
    timestamp: u64,
    nonce: &str,
) -> Result<SignedHeaders, String> {
    for value in [access_key_id, nonce] {
        if value.is_empty() || !value.bytes().all(|byte| (b' '..=b'~').contains(&byte)) {
            return Err("哔哩哔哩签名字段无效".into());
        }
    }
    if access_key_secret.is_empty() {
        return Err("哔哩哔哩签名密钥为空".into());
    }
    let content_md5 = format!("{:x}", Md5::digest(body));
    let timestamp = timestamp.to_string();
    let canonical = [
        format!("x-bili-accesskeyid:{access_key_id}"),
        format!("x-bili-content-md5:{content_md5}"),
        "x-bili-signature-method:HMAC-SHA256".into(),
        format!("x-bili-signature-nonce:{nonce}"),
        "x-bili-signature-version:1.0".into(),
        format!("x-bili-timestamp:{timestamp}"),
    ]
    .join("\n");
    let mut mac = Hmac::<Sha256>::new_from_slice(access_key_secret.as_bytes())
        .map_err(|_| "哔哩哔哩签名密钥无效".to_string())?;
    mac.update(canonical.as_bytes());
    let authorization = hex_lower(&mac.finalize().into_bytes());
    Ok(SignedHeaders {
        content_md5,
        timestamp,
        nonce: nonce.into(),
        access_key_id: access_key_id.into(),
        canonical,
        authorization,
    })
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[usize::from(byte >> 4)] as char);
        output.push(HEX[usize::from(byte & 0xf)] as char);
    }
    output
}

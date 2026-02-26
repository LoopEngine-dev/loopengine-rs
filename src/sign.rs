use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Returns `(timestamp, signature)` for the given request parameters.
///
/// The signature is `HMAC-SHA256` over `"METHOD\nPATH\nTIMESTAMP\nSHA256(body)"`, base64url
/// (no padding), prefixed with `"v1="`.
pub(crate) fn sign_request(secret: &str, method: &str, path: &str, body: &[u8]) -> (String, String) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();

    let body_hash = hex::encode(Sha256::digest(body));
    let canonical = format!("{}\n{}\n{}\n{}", method, path, ts, body_hash);

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(canonical.as_bytes());
    let sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    (ts, format!("v1={sig}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_request_format() {
        let (ts, sig) = sign_request("psk_test", "POST", "/feedback", b"{}");
        assert!(!ts.is_empty(), "timestamp must be non-empty");
        assert!(!sig.is_empty(), "signature must be non-empty");
        assert!(sig.starts_with("v1="), "signature must have v1= prefix, got {sig}");
    }

    #[test]
    fn sign_request_deterministic_same_second() {
        // Two calls within the same second must produce the same signature.
        let body = br#"{"project_id":"proj","message":"hi"}"#;
        let (ts1, sig1) = sign_request("psk_test", "POST", "/feedback", body);
        let (ts2, sig2) = sign_request("psk_test", "POST", "/feedback", body);

        if ts1 == ts2 {
            assert_eq!(sig1, sig2, "same timestamp must yield same signature");
        }
        // If ts1 != ts2 we crossed a second boundary; the signatures are both valid.
    }

    #[test]
    fn sign_request_different_secrets_differ() {
        let body = b"payload";
        let (_, sig1) = sign_request("secret_a", "POST", "/feedback", body);
        let (_, sig2) = sign_request("secret_b", "POST", "/feedback", body);
        assert_ne!(sig1, sig2, "different secrets must produce different signatures");
    }
}

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Verifies that a webhook payload was signed by LoopEngine.
///
/// Pass the raw request body **before** any JSON deserialisation — the signature
/// is computed over the exact bytes received.  Deserialising and re-serialising
/// the payload will produce a different byte sequence and break verification.
///
/// `signature_header` and `timestamp_header` are the values of the
/// `X-LoopEngine-Signature` and `X-LoopEngine-Timestamp` headers respectively.
/// If `max_age_sec > 0`, the timestamp must be within ±`max_age_sec` seconds of
/// now (pass `300` for the recommended 5-minute window, or `0` to skip).
///
/// Returns `true` only when the signature is valid and the timestamp is within
/// the allowed window.
///
/// # Example (Axum)
///
/// ```rust,ignore
/// use axum::{body::Bytes, http::HeaderMap, http::StatusCode};
/// use loopengine::verify_webhook;
///
/// async fn webhook(headers: HeaderMap, body: Bytes) -> StatusCode {
///     let sig  = headers.get("x-loopengine-signature").and_then(|v| v.to_str().ok()).unwrap_or("");
///     let ts   = headers.get("x-loopengine-timestamp").and_then(|v| v.to_str().ok()).unwrap_or("");
///     if !verify_webhook(&std::env::var("LOOPENGINE_WEBHOOK_SECRET").unwrap(), &body, sig, ts, 300) {
///         return StatusCode::UNAUTHORIZED;
///     }
///     let event: serde_json::Value = serde_json::from_slice(&body).unwrap();
///     // handle event …
///     StatusCode::OK
/// }
/// ```
pub fn verify_webhook(
    secret: &str,
    raw_body: &[u8],
    signature_header: &str,
    timestamp_header: &str,
    max_age_sec: u64,
) -> bool {
    if !signature_header.starts_with("v1=") || timestamp_header.is_empty() {
        return false;
    }

    if max_age_sec > 0 {
        let ts: i64 = match timestamp_header.parse() {
            Ok(t) => t,
            Err(_) => return false,
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if (now - ts).abs() > max_age_sec as i64 {
            return false;
        }
    }

    // Decode the raw signature bytes from the "v1=<hex>" header.
    let hex_part = &signature_header[3..];
    let sig_bytes = match hex::decode(hex_part) {
        Ok(b) if !b.is_empty() => b,
        _ => return false,
    };

    // signed content matches server computeSignature: timestamp + "." + rawBody
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(timestamp_header.as_bytes());
    mac.update(b".");
    mac.update(raw_body);

    // mac.verify_slice uses subtle::ConstantTimeEq internally: constant-time.
    mac.verify_slice(&sig_bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    const SECRET: &str = "whsec_live_test_secret";
    const BODY: &[u8] = b"{\"event\":\"feedback.created\",\"id\":\"evt_123\"}";

    fn make_signature(secret: &str, body: &[u8], ts: &str) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(ts.as_bytes());
        mac.update(b".");
        mac.update(body);
        format!("v1={}", hex::encode(mac.finalize().into_bytes()))
    }

    fn now_ts() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }

    #[test]
    fn valid_signature() {
        let ts = now_ts();
        let sig = make_signature(SECRET, BODY, &ts);
        assert!(verify_webhook(SECRET, BODY, &sig, &ts, 300));
    }

    #[test]
    fn tampered_signature() {
        let ts = now_ts();
        let mut sig = make_signature(SECRET, BODY, &ts);
        let len = sig.len();
        sig.replace_range(len - 4.., "aaaa");
        assert!(!verify_webhook(SECRET, BODY, &sig, &ts, 300));
    }

    #[test]
    fn wrong_secret() {
        let ts = now_ts();
        let sig = make_signature("wrong_secret", BODY, &ts);
        assert!(!verify_webhook(SECRET, BODY, &sig, &ts, 300));
    }

    #[test]
    fn altered_body() {
        let ts = now_ts();
        let sig = make_signature(SECRET, BODY, &ts);
        let altered = b"{\"event\":\"feedback.created\",\"id\":\"evt_456\"}";
        assert!(!verify_webhook(SECRET, altered, &sig, &ts, 300));
    }

    #[test]
    fn replay_old_timestamp() {
        let old_ts = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 600)
            .to_string();
        let sig = make_signature(SECRET, BODY, &old_ts);
        assert!(!verify_webhook(SECRET, BODY, &sig, &old_ts, 300));
    }

    #[test]
    fn replay_disabled_with_zero() {
        let old_ts = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 600)
            .to_string();
        let sig = make_signature(SECRET, BODY, &old_ts);
        assert!(verify_webhook(SECRET, BODY, &sig, &old_ts, 0));
    }

    #[test]
    fn missing_v1_prefix() {
        let ts = now_ts();
        let sig = make_signature(SECRET, BODY, &ts);
        assert!(!verify_webhook(SECRET, BODY, &sig[3..], &ts, 300));
    }

    #[test]
    fn empty_signature_header() {
        let ts = now_ts();
        assert!(!verify_webhook(SECRET, BODY, "", &ts, 300));
    }

    #[test]
    fn empty_timestamp_header() {
        let ts = now_ts();
        let sig = make_signature(SECRET, BODY, &ts);
        assert!(!verify_webhook(SECRET, BODY, &sig, "", 300));
    }

    #[test]
    fn non_numeric_timestamp() {
        let ts = now_ts();
        let sig = make_signature(SECRET, BODY, &ts);
        assert!(!verify_webhook(SECRET, BODY, &sig, "not-a-number", 300));
    }
}

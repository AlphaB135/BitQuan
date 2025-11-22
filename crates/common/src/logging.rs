//! Logging utilities and security policy
//!
//! This module provides safe logging helpers that prevent sensitive data leakage.

/// Sanitizes user input before logging to prevent log injection attacks
///
/// Removes control characters (\n, \r, \t) that could be used to forge log entries
/// or manipulate terminal output.
pub fn sanitize_for_log(input: &str) -> String {
    input
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .collect()
}

/// Masks a sensitive string, showing only first and last N characters
///
/// Example: "super_secret_token" -> "supe...oken"
pub fn mask_secret(secret: &str, show_chars: usize) -> String {
    if secret.len() <= show_chars * 2 {
        return "***".to_string();
    }
    format!(
        "{}...{}",
        &secret[..show_chars],
        &secret[secret.len() - show_chars..]
    )
}

/// Returns a fingerprint (hash) of sensitive data for logging
///
/// Useful for debugging without exposing the actual value
pub fn fingerprint(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(data);
    format!("sha256:{}", hex::encode(&hash[..8])) // First 8 bytes
}

/// Redacts common patterns of sensitive data
pub fn redact_secrets(text: &str) -> String {
    use regex::Regex;

    let patterns = vec![
        // JWT tokens (eyJ...)
        (r"eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+", "[REDACTED_JWT]"),
        // Hex keys (64+ hex chars)
        (r"\b[a-fA-F0-9]{64,}\b", "[REDACTED_KEY]"),
        // Base64 encoded (40+ chars)
        (r"[A-Za-z0-9+/]{40,}={0,2}", "[REDACTED_BASE64]"),
    ];

    let mut result = text.to_string();
    for (pattern, replacement) in patterns {
        if let Ok(re) = Regex::new(pattern) {
            result = re.replace_all(&result, replacement).to_string();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_for_log() {
        assert_eq!(sanitize_for_log("hello\nworld"), "helloworld");
        assert_eq!(sanitize_for_log("foo\r\nbar"), "foobar");
        assert_eq!(sanitize_for_log("test\ttab"), "testtab");
        assert_eq!(sanitize_for_log("normal text"), "normal text");
    }

    #[test]
    fn test_mask_secret() {
        assert_eq!(mask_secret("super_secret_token", 4), "supe...oken");
        assert_eq!(mask_secret("short", 4), "***");
        assert_eq!(mask_secret("", 4), "***");
    }

    #[test]
    fn test_fingerprint() {
        let data = b"sensitive data";
        let fp = fingerprint(data);
        assert!(fp.starts_with("sha256:"));
        assert_eq!(fp.len(), "sha256:".len() + 16); // 8 bytes = 16 hex chars
    }

    #[test]
    fn test_redact_secrets() {
        let text = "Token: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature";
        assert!(redact_secrets(text).contains("[REDACTED_JWT]"));

        let text = "Key: 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(redact_secrets(text).contains("[REDACTED_KEY]"));
    }
}

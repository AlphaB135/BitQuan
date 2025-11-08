# bitquan-common

Common utilities and shared code for BitQuan.

## Purpose

This crate contains **logging utilities** that prevent sensitive data leakage and log injection attacks.

**Important:** This crate is currently minimal and may be merged into `bitquan-types` or renamed to `bitquan-logging` in future refactoring.

## Contents

### `logging.rs` - Security-Focused Logging Utilities

Provides safe logging helpers that prevent common security issues:

- **`sanitize_for_log()`** - Removes control characters to prevent log injection
- **`mask_secret()`** - Masks sensitive strings (e.g., API keys, passwords)
- **`fingerprint()`** - Creates SHA-256 hash of sensitive data for debugging
- **`redact_secrets()`** - Auto-redacts common patterns (API keys, tokens, etc.)

### Example Usage

```rust
use bitquan_common::logging::{sanitize_for_log, mask_secret, fingerprint};

// Prevent log injection
let user_input = "admin\nINFO: Fake log entry";
let safe = sanitize_for_log(user_input); // "adminINFO: Fake log entry"

// Mask secrets
let api_key = "sk_live_abc123xyz789";
let masked = mask_secret(api_key, 4); // "sk_l...x789"

// Log fingerprint instead of raw data
let private_key = b"secret_key_bytes";
let fp = fingerprint(private_key); // "sha256:a1b2c3d4..."
```

## Design Philosophy

This crate follows BitQuan's minimalist philosophy:
- ✅ **Single Purpose**: Secure logging only
- ✅ **No Dependencies**: Uses only std + sha2 + hex
- ✅ **Defense in Depth**: Multiple layers of protection

## Future Plans

**Before v0.1.0:**
- Consider renaming to `bitquan-logging` for clarity
- OR merge into `bitquan-types` if it remains minimal

## Security

All functions in this crate are designed with security in mind:
- No unsafe code
- All inputs validated
- Constant-time where applicable (fingerprint hashing)

## License

Apache 2.0 (same as BitQuan)

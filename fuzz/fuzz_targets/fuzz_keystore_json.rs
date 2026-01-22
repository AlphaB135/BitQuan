#![no_main]

//! Fuzzer for wallet keystore JSON parsing.
//!
//! This fuzzer tests the robustness of keystore JSON deserialization
//! by providing randomized, malformed, or unexpected JSON input.
//!
//! Critical security area: Keystores contain private keys, so parser bugs
//! could lead to key leakage or corruption.

use libfuzzer_sys::fuzz_target;
use wallet::keystore::KeystoreFile;

fuzz_target!(|data: &[u8]| {
    // Skip if data is too small to be meaningful JSON
    if data.len() < 2 {
        return;
    }

    // Try to interpret data as UTF-8 string
    let json_str = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip invalid UTF-8
    };

    // Attempt to parse as keystore JSON
    // This should never panic - only return Result::Err for invalid data
    let _ = serde_json::from_str::<KeystoreFile>(json_str);

    // Also test the reverse: try to parse JSON with relaxed validation
    // Some JSON parsers accept trailing commas, comments, etc.
    if let Ok(keystore) = serde_json::from_str::<KeystoreFile>(json_str) {
        // If parsing succeeds, test serialization round-trip
        // This should never panic either
        let _ = keystore.to_json();
        let _ = serde_json::to_string(&keystore);
        let _ = serde_json::to_vec(&keystore);
    }

    // Test with specific attack patterns
    if data.len() >= 32 {
        // Test 1: JSON with null bytes embedded
        let with_nulls: String = json_str
            .chars()
            .flat_map(|c| {
                if c == '{' || c == '}' || c == ',' {
                    vec![c, '\0']
                } else {
                    vec![c]
                }
            })
            .collect();

        let _ = serde_json::from_str::<KeystoreFile>(&with_nulls);

        // Test 2: JSON with excessive nesting (potential stack overflow)
        let mut deeply_nested = json_str.to_string();
        for _ in 0..(data.len() % 10) {
            deeply_nested = format!(r#"{{"nested":{}}}"#, deeply_nested);
        }
        let _ = serde_json::from_str::<KeystoreFile>(&deeply_nested);
    }

    // Test 3: JSON with unicode edge cases
    if data.len() >= 4 {
        // Check for various unicode patterns that might cause issues
        let unicode_test = json_str.chars().all(|c| {
            c.is_ascii() || c.is_alphanumeric() || "!@#$%^&*()_+-=[]{}|;:,.<>?/".contains(c)
        });
        if unicode_test {
            let _ = serde_json::from_str::<KeystoreFile>(json_str);
        }
    }

    // Test 4: Extremely long strings (potential DoS)
    if data.len() > 10000 {
        // Truncate to avoid DoS in the fuzzer itself
        let truncated = &data[..10000];
        if let Ok(s) = std::str::from_utf8(truncated) {
            let _ = serde_json::from_str::<KeystoreFile>(s);
        }
    }
});

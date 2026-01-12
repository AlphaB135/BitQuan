//! Input validation and sanitization for RPC requests

use bitquan_types::error::{Error, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

/// Input validation rules for RPC requests
pub struct InputValidator {
    /// Maximum request size in bytes
    #[allow(dead_code)]
    max_request_size: usize,
    /// Maximum number of parameters per request
    max_parameters: usize,
    /// Maximum string length
    max_string_length: usize,
    /// Maximum array length
    max_array_length: usize,
    /// Maximum nesting depth
    max_nesting_depth: usize,
    /// Allowed JSON-RPC methods
    allowed_methods: HashSet<String>,
    /// Regex patterns for validating strings
    #[allow(dead_code)]
    string_patterns: Vec<Regex>,
    /// Blocked patterns for preventing injection attacks
    blocked_patterns: Vec<Regex>,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl InputValidator {
    /// Create a new input validator with default security settings
    pub fn new() -> Self {
        let allowed_methods = vec![
            "getblockcount",
            "getblockhash",
            "getblock",
            "getblockchaininfo",
            "getrawmempool",
            "getmempoolentry",
            "getrawtransaction",
            "sendrawtransaction",
            "getpeerinfo",
            "getnetworkinfo",
            "validateaddress",
            "getbalance",
            "listunspent",
            "listsinceblock",
            "gettransaction",
            "getblocktemplate",
            "submitblock",
            "estimatefee",
            "getmininginfo",
            "prioritisetransaction",
            "generate",           // CPU mining for testing
            "getwork",            // Get mining work
            "submitwork",         // Submit mining work
            "getbestblockhash",   // Get best block hash
            "submittransaction",  // Submit transaction
            "sync",               // Get sync status
            "getpoolstats",       // Get mining pool stats
            "getminerstats",      // Get miner stats
            "createpayout",       // Create payout
            "getnetworkstatus",   // Get network status
        ];

        let mut validator = Self {
            max_request_size: 1_048_576,  // 1 MB
            max_parameters: 100,          // Max 100 parameters
            max_string_length: 1_048_576, // 1 MB per string
            max_array_length: 10_000,     // Max 10,000 items per array
            max_nesting_depth: 10,        // Max 10 levels deep
            allowed_methods: allowed_methods.into_iter().map(|s| s.to_string()).collect(),
            string_patterns: Vec::new(),
            blocked_patterns: Vec::new(),
        };

        // Add common injection patterns to block
        validator.add_blocked_patterns();

        validator
    }

    /// Add a blocked pattern for input validation
    pub fn add_blocked_pattern(&mut self, pattern: &str) -> Result<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| Error::Invalid(format!("Invalid regex pattern: {}", e)))?;
        self.blocked_patterns.push(regex);
        Ok(())
    }

    /// Add commonly blocked patterns for injection prevention
    fn add_blocked_patterns(&mut self) {
        let patterns = vec![
            r"(?i)<script[^>]*>.*?</script>",         // XSS
            r"(?i)javascript:",                       // JavaScript protocol
            r"(?i)vbscript:",                         // VBScript protocol
            r"(?i)on\w+\s*=",                         // Event handlers
            r"(?i)eval\s*\(",                         // eval()
            r"(?i)exec\s*\(",                         // exec()
            r"(?i)system\s*\(",                       // system()
            r"(?i)shell_exec\s*\(",                   // shell_exec()
            r"(?i)passthru\s*\(",                     // passthru()
            r"(?i)base64_decode\s*\(",                // base64_decode()
            r"(?i)file_get_contents\s*\(",            // file_get_contents()
            r"(?i)file_put_contents\s*\(",            // file_put_contents()
            r"(?i)fopen\s*\(",                        // fopen()
            r"(?i)fwrite\s*\(",                       // fwrite()
            r"(?i)curl_exec\s*\(",                    // curl_exec()
            r"(?i)proc_open\s*\(",                    // proc_open()
            r"(?i)sql\s*(insert|update|delete|drop)", // SQL keywords
            r"(?i)union\s+select",                    // SQL Union
            r"(?i)--|#",                              // SQL comments
            r"(?i)'\s*;\s*",                          // SQL command separator
            r"(?i)<\?php",                            // PHP tags
            r"(?i)<%\s*=.*?%>",                       // ASP tags
            r"(?i){{.*?}}",                           // Template injection
            r"(?i)\$\{.*?}",                          // Template injection
            r"(?i)cmd\s*\|",                          // Command injection
            r"(?i)&&\s*\|\|\|",                       // Command chaining
            r"(?i)[`'].*[`']",                        // Backtick/string execution
            r"(?i)\\x[0-9a-f]{2}",                    // Hex encoding
            r"(?i)\\u[0-9a-f]{4}",                    // Unicode encoding
            r"(?i)%[0-9a-f]{2}",                      // URL encoding
            r"(?i)path\s*\.\./",                      // Path traversal
            r"(?i)\.\./.*\.\./",                      // Deep path traversal
            r"(?i)[/\\]etc[/\\]",                     // System file access
            r"(?i)[/\\]bin[/\\]",                     // Binary access
            r"(?i)[/\\]usr[/\\]",                     // System directory
            r"(?i)[/\\]var[/\\]",                     // Var directory
            r"(?i)[/\\]tmp[/\\]",                     // Temp directory
        ];

        for pattern in patterns {
            if let Ok(regex) = Regex::new(pattern) {
                self.blocked_patterns.push(regex);
            }
        }
    }

    /// Validate a JSON-RPC request
    pub fn validate_request(&self, request: &Value) -> Result<()> {
        // Check if request is a valid JSON-RPC 2.0 request
        self.validate_jsonrpc_format(request)?;

        // Validate method
        let method = request
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Invalid("Missing or invalid method".to_string()))?;

        self.validate_method(method)?;

        // Validate parameters
        if let Some(params) = request.get("params") {
            self.validate_parameters(params)?;
        }

        // Validate ID if present
        if let Some(id) = request.get("id") {
            self.validate_id(id)?;
        }

        // Check for blocked patterns in the entire request
        self.validate_for_injections(request)?;

        Ok(())
    }

    /// Validate JSON-RPC 2.0 format
    fn validate_jsonrpc_format(&self, request: &Value) -> Result<()> {
        // Check if jsonrpc field exists and is "2.0"
        match request.get("jsonrpc") {
            Some(version) => {
                if version.as_str() != Some("2.0") {
                    return Err(Error::Invalid("Only JSON-RPC 2.0 is supported".to_string()));
                }
            }
            None => return Err(Error::Invalid("Missing jsonrpc field".to_string())),
        }

        Ok(())
    }

    /// Validate method name
    fn validate_method(&self, method: &str) -> Result<()> {
        // Check method length
        if method.is_empty() || method.len() > 100 {
            return Err(Error::Invalid("Invalid method length".to_string()));
        }

        // Check if method is allowed
        if !self.allowed_methods.contains(method) {
            return Err(Error::Invalid(format!(
                "Method '{}' is not allowed",
                method
            )));
        }

        // Check method characters
        if !method
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return Err(Error::Invalid(
                "Method contains invalid characters".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate parameters
    fn validate_parameters(&self, params: &Value) -> Result<()> {
        match params {
            Value::Null => Ok(()), // No parameters is valid
            Value::Array(array) => {
                if array.len() > self.max_parameters {
                    return Err(Error::Invalid(format!(
                        "Too many parameters: {} > {}",
                        array.len(),
                        self.max_parameters
                    )));
                }

                for (i, param) in array.iter().enumerate() {
                    self.validate_value(param, i)?;
                }
                Ok(())
            }
            Value::Object(_) => {
                // Named parameters - recursively validate
                self.validate_value(params, 0)?;
                Ok(())
            }
            _ => Err(Error::Invalid(
                "Parameters must be an array or object".to_string(),
            )),
        }
    }

    /// Validate individual values recursively
    fn validate_value(&self, value: &Value, depth: usize) -> Result<()> {
        if depth > self.max_nesting_depth {
            return Err(Error::Invalid("Maximum nesting depth exceeded".to_string()));
        }

        match value {
            Value::String(s) => {
                self.validate_string(s)?;
            }
            Value::Array(array) => {
                if array.len() > self.max_array_length {
                    return Err(Error::Invalid(format!(
                        "Array too long: {} > {}",
                        array.len(),
                        self.max_array_length
                    )));
                }

                for item in array {
                    self.validate_value(item, depth + 1)?;
                }
            }
            Value::Object(object) => {
                for (key, val) in object {
                    self.validate_string(key)?;
                    self.validate_value(val, depth + 1)?;
                }
            }
            Value::Number(num) => {
                if num.is_f64()
                    && (num.as_f64().unwrap_or(0.0).is_infinite()
                        || num.as_f64().unwrap_or(0.0).is_nan())
                {
                    return Err(Error::Invalid(
                        "Invalid number: NaN or infinite".to_string(),
                    ));
                }
            }
            Value::Bool(_) => {}
            Value::Null => {}
        }

        Ok(())
    }

    /// Validate string content
    fn validate_string(&self, s: &str) -> Result<()> {
        // Check string length
        if s.len() > self.max_string_length {
            return Err(Error::Invalid(format!(
                "String too long: {} > {}",
                s.len(),
                self.max_string_length
            )));
        }

        // Check for blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(s) {
                return Err(Error::Invalid(
                    "String contains blocked content".to_string(),
                ));
            }
        }

        // Check for null bytes (potential attacks)
        if s.contains('\0') {
            return Err(Error::Invalid("String contains null bytes".to_string()));
        }

        // Check for control characters (except common whitespace)
        for ch in s.chars() {
            if ch.is_control() && !matches!(ch, '\t' | '\n' | '\r') {
                return Err(Error::Invalid(
                    "String contains control characters".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validate request ID
    fn validate_id(&self, id: &Value) -> Result<()> {
        match id {
            Value::String(s) => {
                if s.len() > 100 {
                    return Err(Error::Invalid("ID string too long".to_string()));
                }
            }
            Value::Number(_) => {}
            Value::Null => {}
            _ => {
                return Err(Error::Invalid(
                    "ID must be string, number, or null".to_string(),
                ))
            }
        }

        Ok(())
    }

    /// Validate against injection patterns
    fn validate_for_injections(&self, request: &Value) -> Result<()> {
        let request_str = serde_json::to_string(request)
            .map_err(|e| Error::Invalid(format!("Invalid JSON: {}", e)))?;

        // Check for blocked patterns in the entire request
        for pattern in &self.blocked_patterns {
            if pattern.is_match(&request_str) {
                return Err(Error::Invalid(
                    "Request contains potentially malicious content".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Sanitize input by removing or escaping dangerous characters
    pub fn sanitize_string(&self, input: &str) -> String {
        let mut result = String::with_capacity(input.len());

        for ch in input.chars() {
            match ch {
                // Remove null bytes and most control characters
                '\0' => {}
                c if c.is_control() && !matches!(c, '\t' | '\n' | '\r') => {}

                // Escape potentially dangerous characters
                '<' => result.push_str("&lt;"),
                '>' => result.push_str("&gt;"),
                '&' => result.push_str("&amp;"),
                '"' => result.push_str("&quot;"),
                '\'' => result.push_str("&#x27;"),

                // Allow safe characters
                _ => result.push(ch),
            }
        }

        result
    }

    /// Validate and sanitize a string in one step
    pub fn clean_string(&self, input: &str) -> Result<String> {
        self.validate_string(input)?;
        Ok(self.sanitize_string(input))
    }

    /// Create a strict validator for high-security environments
    pub fn strict() -> Self {
        let mut validator = Self {
            max_request_size: 256_000,  // 256 KB
            max_parameters: 50,         // Max 50 parameters
            max_string_length: 100_000, // 100 KB per string
            max_array_length: 1_000,    // Max 1,000 items per array
            max_nesting_depth: 5,       // Max 5 levels deep
            allowed_methods: HashSet::new(),
            string_patterns: Vec::new(),
            blocked_patterns: Vec::new(),
        };

        // Only allow read-only methods in strict mode
        let allowed_methods = vec![
            "getblockcount",
            "getblockhash",
            "getblock",
            "getblockchaininfo",
            "getrawmempool",
            "gettransaction",
            "getpeerinfo",
            "getnetworkinfo",
            "validateaddress",
            "getbalance",
            "listunspent",
            "listsinceblock",
            "getblocktemplate",
            "estimatefee",
            "getmininginfo",
        ];

        validator.allowed_methods = allowed_methods.into_iter().map(|s| s.to_string()).collect();
        validator.add_blocked_patterns();
        validator
    }

    /// Create a permissive validator for development environments
    pub fn permissive() -> Self {
        let mut validator = Self {
            max_request_size: 5_242_880,  // 5 MB
            max_parameters: 500,          // Max 500 parameters
            max_string_length: 5_242_880, // 5 MB per string
            max_array_length: 100_000,    // Max 100,000 items per array
            max_nesting_depth: 20,        // Max 20 levels deep
            allowed_methods: HashSet::new(),
            string_patterns: Vec::new(),
            blocked_patterns: Vec::new(),
        };

        // Allow all common RPC methods in permissive mode
        let allowed_methods = vec![
            "getblockcount",
            "getblockhash",
            "getblock",
            "getblockchaininfo",
            "getrawmempool",
            "getmempoolentry",
            "getrawtransaction",
            "sendrawtransaction",
            "getpeerinfo",
            "getnetworkinfo",
            "validateaddress",
            "getbalance",
            "listunspent",
            "listsinceblock",
            "gettransaction",
            "getblocktemplate",
            "submitblock",
            "estimatefee",
            "getmininginfo",
            "prioritisetransaction",
            "addnode",
            "removenode",
            "disconnectnode",
            "setban",
            "listbanned",
            "clearbanned",
            "stop",
            "getmemoryinfo",
            "getrpcinfo",
            "help",
            "generatetoaddress",
            "generate",           // CPU mining for testing
            "getwork",            // Get mining work
            "submitwork",         // Submit mining work
            "getbestblockhash",   // Get best block hash
            "submittransaction",  // Submit transaction
            "sync",               // Get sync status
            "getpoolstats",       // Get mining pool stats
            "getminerstats",      // Get miner stats
            "createpayout",       // Create payout
            "getnetworkstatus",   // Get network status
            "getnewaddress",
            "getaddressinfo",
            "validateaddress",
            "createmultisig",
            "addmultisigaddress",
            "signmessage",
            "verifymessage",
            "settxfee",
            "getreceivedbyaddress",
            "listreceivedbyaddress",
            "listtransactions",
            "listlockunspent",
            "lockunspent",
            "importprivkey",
            "importaddress",
            "importwallet",
            "importprunedfunds",
            "keypoolrefill",
            "walletlock",
            "walletpassphrase",
            "backupwallet",
            "importwallet",
            "dumpprivkey",
            "importprivkey",
            "dumpwallet",
            "encryptwallet",
        ];

        validator.allowed_methods = allowed_methods.into_iter().map(|s| s.to_string()).collect();
        validator.add_blocked_patterns();
        validator
    }
}

/// Global input validator instance.
///
/// Uses OnceLock for thread-safe lazy initialization without unsafe code.
/// OnceLock is stable since Rust 1.70, so it works with MSRV 1.79.
///
/// -- Linus-style refactor: killed `static mut` and all unsafe blocks.
/// Had to use OnceLock instead of LazyLock because project MSRV is 1.79
/// and LazyLock requires 1.80. Still safe, just slightly more verbose.
static GLOBAL_VALIDATOR: std::sync::OnceLock<InputValidator> = std::sync::OnceLock::new();

/// Get the global input validator.
///
/// Lazily initializes the default validator on first call.
/// This is safe, fast, and doesn't require unsafe blocks.
pub fn get_validator() -> &'static InputValidator {
    GLOBAL_VALIDATOR.get_or_init(InputValidator::default)
}

/// Initialize the global validator with custom settings.
///
/// IMPORTANT: This can only be called ONCE, and only BEFORE the first
/// call to get_validator(). After that, it's immutable.
///
/// Returns true if initialization succeeded, false if already initialized.
pub fn init_validator(validator: InputValidator) -> bool {
    GLOBAL_VALIDATOR.set(validator).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_safe_request() {
        let validator = InputValidator::default();

        let safe_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": [],
            "id": 1
        });

        assert!(validator.validate_request(&safe_request).is_ok());
    }

    #[test]
    fn test_block_xss_injection() {
        let validator = InputValidator::default();

        let malicious_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": ["<script>alert('xss')</script>"],
            "id": 1
        });

        assert!(validator.validate_request(&malicious_request).is_err());
    }

    #[test]
    fn test_block_sql_injection() {
        let validator = InputValidator::default();

        let malicious_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": ["'; DROP TABLE users; --"],
            "id": 1
        });

        assert!(validator.validate_request(&malicious_request).is_err());
    }

    #[test]
    fn test_max_parameters_enforced() {
        let validator = InputValidator::strict(); // Lower limit for testing

        let mut params = Vec::new();
        for i in 0..100 {
            // More than the strict limit
            params.push(Value::String(format!("param_{}", i)));
        }

        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": params,
            "id": 1
        });

        assert!(validator.validate_request(&request).is_err());
    }

    #[test]
    fn test_string_sanitization() {
        let validator = InputValidator::default();

        let dangerous_string = "<script>alert('xss')</script>";
        let sanitized = validator.sanitize_string(dangerous_string);

        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(sanitized.contains("&lt;"));
        assert!(sanitized.contains("&gt;"));
    }

    #[test]
    fn test_strict_vs_permissive_modes() {
        let strict_validator = InputValidator::strict();
        let permissive_validator = InputValidator::permissive();

        let write_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sendrawtransaction",
            "params": ["raw_tx_data"],
            "id": 1
        });

        // Strict validator should reject write operations
        assert!(strict_validator.validate_request(&write_request).is_err());

        // Permissive validator should accept them
        assert!(permissive_validator
            .validate_request(&write_request)
            .is_ok());
    }
}

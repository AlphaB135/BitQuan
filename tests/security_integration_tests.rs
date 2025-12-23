//! Security Integration Tests for BitQuan
//! ทดสอบการทำงานของฟีเจอร์ความปลอดภัยต่างๆ

use bitquan_rpc::{server::{SecurityEvent, SecurityEventType, SecuritySeverity}, validation::InputValidator};
use serde_json::json;

#[cfg(test)]
mod security_tests {
    use super::*;

    #[test]
    fn test_security_event_creation() {
        // ทดสอบการสร้าง security event
        let event = SecurityEvent::new(
            "127.0.0.1".to_string(),
            SecurityEventType::RateLimitExceeded,
            SecuritySeverity::Medium,
            json!({"action": "rate_limit_test"}),
        );

        assert_eq!(event.client_ip, "127.0.0.1");
        assert!(matches!(event.event_type, SecurityEventType::RateLimitExceeded));
        assert!(matches!(event.severity, SecuritySeverity::Medium));
    }

    #[test]
    fn test_security_event_serialization() {
        // ทดสอบการแปลง event เป็น JSON
        let event = SecurityEvent::new(
            "192.168.1.1".to_string(),
            SecurityEventType::AuthenticationFailed,
            SecuritySeverity::High,
            json!({"reason": "invalid_credentials"}),
        );

        let json = event.to_json();
        assert_eq!(json["client_ip"], "192.168.1.1");
        assert!(json["event_type"].as_str().unwrap().contains("AuthenticationFailed"));
        assert!(json["severity"].as_str().unwrap().contains("High"));
    }

    #[test]
    fn test_security_event_alerting() {
        // ทดสอบการ trigger alerts
        let high_severity_event = SecurityEvent::new(
            "10.0.0.1".to_string(),
            SecurityEventType::InjectionAttempt,
            SecuritySeverity::High,
            json!({"pattern": "sql_injection"}),
        );
        assert!(high_severity_event.should_alert());

        let info_severity_event = SecurityEvent::new(
            "10.0.0.2".to_string(),
            SecurityEventType::ConnectionEstablished,
            SecuritySeverity::Info,
            json!({"status": "connected"}),
        );
        assert!(!info_severity_event.should_alert());
    }

    #[test]
    fn test_input_validator_safe_requests() {
        // ทดสอบ request ที่ปลอดภัย
        let validator = InputValidator::default();

        let safe_request = json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": [],
            "id": 1
        });

        assert!(validator.validate_request(&safe_request).is_ok());
    }

    #[test]
    fn test_input_validator_blocks_xss() {
        // ทดสอบการบล็อก XSS
        let validator = InputValidator::default();

        let xss_request = json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": ["<script>alert('xss')</script>"],
            "id": 1
        });

        assert!(validator.validate_request(&xss_request).is_err());
    }

    #[test]
    fn test_input_validator_blocks_sql_injection() {
        // ทดสอบการบล็อก SQL Injection
        let validator = InputValidator::default();

        let sql_injection_request = json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": ["'; DROP TABLE users; --"],
            "id": 1
        });

        assert!(validator.validate_request(&sql_injection_request).is_err());
    }

    #[test]
    fn test_input_validator_blocks_command_injection() {
        // ทดสอบการบล็อก Command Injection
        let validator = InputValidator::default();

        let cmd_injection_request = json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": ["`rm -rf /`"],
            "id": 1
        });

        assert!(validator.validate_request(&cmd_injection_request).is_err());
    }

    #[test]
    fn test_input_validator_method_validation() {
        // ทดสอบการ validate method names
        let validator = InputValidator::default();

        // Valid method
        let valid_request = json!({
            "jsonrpc": "2.0",
            "method": "getblockhash",
            "params": [1000],
            "id": 1
        });
        assert!(validator.validate_request(&valid_request).is_ok());

        // Invalid method (not in allowlist)
        let invalid_request = json!({
            "jsonrpc": "2.0",
            "method": "malicious_method",
            "params": [],
            "id": 1
        });
        assert!(validator.validate_request(&invalid_request).is_err());
    }

    #[test]
    fn test_input_validator_parameter_limits() {
        // ทดสอบการจำกัดจำนวน parameters
        let validator = InputValidator::strict(); // ใช้ strict mode

        // Too many parameters
        let mut params = Vec::new();
        for i in 0..200 { // เกิน limit ของ strict mode
            params.push(json!(i));
        }

        let too_many_params_request = json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": params,
            "id": 1
        });

        assert!(validator.validate_request(&too_many_params_request).is_err());
    }

    #[test]
    fn test_input_validator_nesting_depth() {
        // ทดสอบการจำกัด nesting depth
        let validator = InputValidator::strict();

        // Create deeply nested object
        let mut nested = json!({});
        for _ in 0..10 { // ควรจะถูกปฏิเสธใน strict mode
            nested = json!({"nested": nested});
        }

        let deep_nesting_request = json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": [nested],
            "id": 1
        });

        assert!(validator.validate_request(&deep_nesting_request).is_err());
    }

    #[test]
    fn test_string_sanitization() {
        // ทดสอบการ sanitize strings
        let validator = InputValidator::default();

        let dangerous_string = "<script>alert('xss')</script>";
        let sanitized = validator.sanitize_string(dangerous_string);

        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(sanitized.contains("&lt;"));
        assert!(sanitized.contains("&gt;"));
    }

    #[test]
    fn test_validator_modes() {
        // ทดสอบความแตกต่างระหว่าง validator modes
        let strict_validator = InputValidator::strict();
        let permissive_validator = InputValidator::permissive();

        // Write operation (ควรถูกบล็อกใน strict mode)
        let write_request = json!({
            "jsonrpc": "2.0",
            "method": "sendrawtransaction",
            "params": ["raw_transaction_data"],
            "id": 1
        });

        assert!(strict_validator.validate_request(&write_request).is_err());
        assert!(permissive_validator.validate_request(&write_request).is_ok());
    }

    #[test]
    fn test_null_byte_removal() {
        // ทดสอบการจัดการ null bytes
        let validator = InputValidator::default();

        let null_byte_string = "test\x00\x00string";
        let sanitized = validator.sanitize_string(null_byte_string);

        assert!(!sanitized.contains('\x00'));
        assert_eq!(sanitized, "teststring");
    }

    #[test]
    fn test_control_character_handling() {
        // ทดสอบการจัดการ control characters
        let validator = InputValidator::default();

        let control_string = "test\x01\x02\x03string";
        let sanitized = validator.sanitize_string(control_string);

        // Control characters ควรถูกลบออก
        assert_eq!(sanitized, "teststring");
    }

    #[tokio::test]
    async fn test_security_event_logging() {
        // ทดสอบการ log security events
        use tracing_test::traced_test;

        let event = SecurityEvent::new(
            "test-ip".to_string(),
            SecurityEventType::SuspiciousRequest,
            SecuritySeverity::Medium,
            json!({"test": "security_logging"}),
        );

        // ใช้ tracing_test สำหรับ test logging (ต้องเพิ่ม dependency)
        // traced_test!(async {
        //     event.log();
        // });

        // Simple test สำหรับตอนนี้
        assert!(event.to_json()["test"] == "security_logging");
    }

    #[test]
    fn test_comprehensive_security_validation() {
        // ทดสอบรวม security validations หลายๆ อย่าง
        let validator = InputValidator::default();

        let test_cases = vec![
            // ปลอดภัย
            (json!({
                "jsonrpc": "2.0",
                "method": "getblockcount",
                "params": [],
                "id": 1
            }), true),

            // XSS
            (json!({
                "jsonrpc": "2.0",
                "method": "getblockcount",
                "params": ["<img src=x onerror=alert('xss')>"],
                "id": 1
            }), false),

            // SQL Injection
            (json!({
                "jsonrpc": "2.0",
                "method": "getblockcount",
                "params": ["' OR 1=1 --"],
                "id": 1
            }), false),

            // Invalid method
            (json!({
                "jsonrpc": "2.0",
                "method": "eval",
                "params": ["malicious_code()"],
                "id": 1
            }), false),

            // Path traversal
            (json!({
                "jsonrpc": "2.0",
                "method": "getblockcount",
                "params": ["../../../etc/passwd"],
                "id": 1
            }), false),
        ];

        for (request, should_pass) in test_cases {
            let result = validator.validate_request(&request);
            assert_eq!(result.is_ok(), should_pass, "Failed for request: {}", request);
        }
    }
}

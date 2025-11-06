//! Audit report schema validation test.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AuditReport {
    status: AuditStatus,
    findings: Vec<Finding>,
    sha: String,
    tag: String,
    auditor: String,
    date: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum AuditStatus {
    Pass,
    Fail,
}

#[derive(Debug, Serialize, Deserialize)]
struct Finding {
    severity: Severity,
    title: String,
    description: String,
    file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
    recommendation: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[test]
fn test_audit_report_schema_valid_pass() {
    let json = r#"
    {
        "status": "pass",
        "findings": [],
        "sha": "abc123def456",
        "tag": "v1.0.0-rc1",
        "auditor": "Acme Security Ltd",
        "date": "2025-11-06T10:00:00Z"
    }
    "#;

    let report: AuditReport = serde_json::from_str(json).expect("Valid schema");
    assert_eq!(report.status, AuditStatus::Pass);
    assert!(report.findings.is_empty());
}

#[test]
fn test_audit_report_schema_valid_fail_with_findings() {
    let json = r#"
    {
        "status": "fail",
        "findings": [
            {
                "severity": "high",
                "title": "Potential integer overflow",
                "description": "Unchecked arithmetic in difficulty calculation",
                "file": "crates/consensus/src/asert.rs",
                "line": 42,
                "recommendation": "Use checked_add/checked_mul"
            },
            {
                "severity": "medium",
                "title": "Missing rate limit",
                "description": "RPC endpoint lacks rate limiting",
                "file": "crates/rpc/src/methods.rs",
                "recommendation": "Apply rate limiter middleware"
            }
        ],
        "sha": "def789abc012",
        "tag": "v1.0.0-rc2",
        "auditor": "Security Experts Inc",
        "date": "2025-11-06T12:30:00Z"
    }
    "#;

    let report: AuditReport = serde_json::from_str(json).expect("Valid schema");
    assert_eq!(report.status, AuditStatus::Fail);
    assert_eq!(report.findings.len(), 2);
    assert_eq!(report.findings[0].title, "Potential integer overflow");
}

#[test]
fn test_audit_report_schema_missing_required_field() {
    let json = r#"
    {
        "status": "pass",
        "findings": [],
        "sha": "abc123",
        "tag": "v1.0.0"
    }
    "#;

    let result: Result<AuditReport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail with missing auditor and date");
}

#[test]
fn test_audit_report_schema_invalid_status() {
    let json = r#"
    {
        "status": "unknown",
        "findings": [],
        "sha": "abc123",
        "tag": "v1.0.0",
        "auditor": "Test",
        "date": "2025-11-06T10:00:00Z"
    }
    "#;

    let result: Result<AuditReport, _> = serde_json::from_str(json);
    assert!(result.is_err(), "Should fail with invalid status");
}

#[test]
fn test_audit_report_roundtrip() {
    let report = AuditReport {
        status: AuditStatus::Pass,
        findings: vec![Finding {
            severity: Severity::Info,
            title: "Code quality note".to_string(),
            description: "Consider refactoring for clarity".to_string(),
            file: "crates/node/src/main.rs".to_string(),
            line: Some(100),
            recommendation: "Extract helper function".to_string(),
        }],
        sha: "123abc456def".to_string(),
        tag: "v1.0.0".to_string(),
        auditor: "Internal Review".to_string(),
        date: "2025-11-06T00:00:00Z".to_string(),
    };

    let json = serde_json::to_string_pretty(&report).expect("Serialize");
    let parsed: AuditReport = serde_json::from_str(&json).expect("Deserialize");

    assert_eq!(parsed.status, AuditStatus::Pass);
    assert_eq!(parsed.findings.len(), 1);
    assert_eq!(parsed.findings[0].title, "Code quality note");
}

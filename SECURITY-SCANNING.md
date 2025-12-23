# BitQuan Security Scanning Guide

## ภาพรวมระบบ Security Scanning

BitQuan มีระบบตรวจสอบความปลอดภัยแบบครบวงจรที่ทำงานอัตโนมัติ ประกอบด้วย:

### 🔍 ประเภทการสแกน

#### 1. **Static Application Security Testing (SAST)**
- **CodeQL Analysis**: วิเคราะห์ code หา vulnerabilities
- **Cargo Clippy**: Security-focused linting
- **Cargo Audit**: ตรวจสอบ security advisories ใน dependencies

#### 2. **Dependency Security**
- **Cargo Deny**: License และ dependency bans
- **Dependabot**: Automatic dependency updates
- **Supply Chain Analysis**: SBOM generation

#### 3. **Secret Scanning**
- **TruffleHog**: ตรวจสอบ hardcoded secrets
- **Cargo Secret**: Rust-specific secret detection
- **GitHub Secret Scanning**: Platform-level secret detection

#### 4. **Runtime Security**
- **Security Event Logging**: Real-time monitoring
- **Input Validation**: Prevent injection attacks
- **Rate Limiting**: DDoS protection

## 🚀 การติดตั้งและการตั้งค่า

### Dependencies Required

```bash
# Install security scanning tools
cargo install cargo-audit cargo-deny cargo-secret
cargo install cargo-tarpaulin  # For coverage
```

### Environment Variables

```bash
# Security monitoring
export SECURITY_LOG_FILE="/var/log/bitquan/security.log"
export ALERT_THRESHOLD=10
export SCAN_INTERVAL=300

# Alerting (optional)
export ALERT_WEBHOOK="https://hooks.slack.com/your-webhook-url"
export SCORECARD_READ_TOKEN="your-github-token"
```

## 📋 การสแกนอัตโนมัติ

### GitHub Actions Workflows

#### 1. **Security Scan Workflow** (`.github/workflows/security-scan.yml`)
รันทุกวันเวลา 2:00 UTC และเมื่อมีการ push/PR:

- Security audit (cargo audit)
- License checking (cargo deny)
- Clippy security lints
- Secret scanning (trufflehog)
- Dependency confusion check
- Malware detection (ClamAV)
- YARA rule analysis

#### 2. **CodeQL Analysis** (`.github/workflows/codeql-analysis.yml`)
รันทุกวันเวลา 3:00 UTC:

- Static analysis for security vulnerabilities
- Custom security queries
- OSSF Scorecard analysis
- Dependency review for PRs

### Local Development

```bash
# Run security audit
cargo audit

# Check dependencies
cargo deny check

# Run security-focused tests
cargo test security_integration_tests

# Start security monitoring
./scripts/security-monitor.sh
```

## 🛡️ ฟีเจอร์ความปลอดภัยที่ implement

### 1. **Input Validation System**

```rust
use bitquan_rpc::validation::InputValidator;

let validator = InputValidator::default();
let request = json!({
    "jsonrpc": "2.0",
    "method": "getblockcount",
    "params": [],
    "id": 1
});

assert!(validator.validate_request(&request).is_ok());
```

**ป้องกันการโจมตี:**
- XSS (Cross-Site Scripting)
- SQL Injection
- Command Injection
- Path Traversal

### 2. **Rate Limiting**

```rust
// Token bucket algorithm
if !check_rate_limit(client_ip, &limiter, &config) {
    // Block request
    return Err(RateLimitExceeded);
}
```

**ป้องกันการโจมตี:**
- DDoS attacks
- Brute force attacks
- API abuse

### 3. **Security Event Logging**

```rust
let event = SecurityEvent::new(
    client_ip,
    SecurityEventType::SuspiciousRequest,
    SecuritySeverity::High,
    json!({"pattern": "injection_attempt"})
);

event.log(); // ส่งไปยัง logging system
```

**ตรวจสอบ:**
- Authentication failures
- Rate limit violations
- Input validation failures
- Suspicious requests

## 📊 Security Metrics และ Monitoring

### Key Metrics

1. **Authentication Failures**: จำนวนครั้งที่ authentication ล้มเหลว
2. **Rate Limit Violations**: คำขอที่ถูก block
3. **Input Validation Failures**: Request ที่ไม่ผ่าน validation
4. **Security Alerts**: Events ระดับ High/Critical

### Dashboard Monitoring

```bash
# ตรวจสอบ security events ล่าสุด
tail -f /var/log/bitquan/security.log | grep -E "(WARNING|CRITICAL)"

# สรุป security events ในชั่วโมงล่าสุด
grep "$(date '+%Y-%m-%d %H:')" /var/log/bitquan/security.log | \
    awk '{print $4}' | sort | uniq -c | sort -nr
```

## 🚨 Alerting และ Incident Response

### Alert Thresholds

- **Critical**: 5+ events ใน 5 นาที
- **High**: 10+ events ใน 1 ชั่วโมง
- **Medium**: 25+ events ใน 24 ชั่วโมง

### Response Procedures

1. **Immediate Response** (0-5 นาที):
   - ตรวจสอบ alert source
   - ประเมิน impact
   - ส่ง notification ให้ security team

2. **Investigation** (5-30 นาที):
   - วิเคราะห์ logs
   - ตรวจสอบ affected systems
   - ระบุ root cause

3. **Containment** (30-60 นาที):
   - Block malicious IPs
   - ปรับ rate limits
   - อัพเดต security rules

4. **Recovery** (1-24 ชั่วโมง):
   - Patch vulnerabilities
   - Update configurations
   - Monitor for recurrence

## 🔧 Configuration Options

### Input Validator Modes

```rust
// Strict mode - production
let validator = InputValidator::strict();

// Permissive mode - development
let validator = InputValidator::permissive();

// Custom configuration
let validator = InputValidator::new()
    .with_max_parameters(50)
    .with_max_string_length(100000);
```

### Rate Limiting Configuration

```rust
let config = RpcConfig {
    rate_limit_requests: 100,      // 100 requests per window
    rate_limit_window: 60,         // 1 minute window
    cooldown_duration: Duration::from_secs(300), // 5 minute cooldown
    // ... other config
};
```

## 📈 Security Best Practices

### Code Development

1. **Input Validation**: ตรวจสอบทุก input ก่อน processing
2. **Error Handling**: ไม่ expose sensitive information ใน error messages
3. **Logging**: Log security events แต่ไม่ include sensitive data
4. **Dependencies**: ใช้ pinned versions สำหรับ production

### Infrastructure Security

1. **Network Isolation**: ใช้ firewalls และ network segmentation
2. **TLS Everywhere**: HTTPS สำหรับทุก communications
3. **Secrets Management**: ไม่ hardcoded secrets ใน code
4. **Regular Updates**: Keep dependencies และ systems up-to-date

### Operational Security

1. **Monitoring**: ตรวจสอบ security logs อย่างสม่ำเสมอ
2. **Incident Response**: มี response plan ที่ชัดเจน
3. **Backup Security**: Encrypt และ test backups อย่างสม่ำเสมอ
4. **Access Control**: ใช้ principle of least privilege

## 🧪 Testing Security

### Running Security Tests

```bash
# Run all security integration tests
cargo test security_integration_tests

# Test with sanitizer (nightly only)
cargo +nightly test -Z build-std --target x86_64-unknown-linux-gnu \
    -Z sanitizer=address security_integration_tests

# Fuzz testing (if available)
cd fuzz && cargo +nightly fuzz run fuzz_target_1
```

### Security Test Coverage

- [ ] Input validation bypass attempts
- [ ] Rate limit evasion techniques
- [ ] Authentication bypass attempts
- [ ] Privilege escalation scenarios
- [ ] Resource exhaustion attacks

## 📚 References และ Resources

### Security Tools Documentation

- [Cargo Audit](https://github.com/RustSec/rustsec-cargo)
- [Cargo Deny](https://embarkstudios.github.io/cargo-deny/)
- [CodeQL](https://codeql.github.com/)
- [TruffleHog](https://github.com/trufflesecurity/trufflehog)

### Security Guidelines

- [Rust Security Guidelines](https://doc.rust-lang.org/book/ch19-06-macros.html?highlight=security#macros)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)

### Security Communities

- [RustSec](https://rustsec.org/)
- [OWASP Rust Project](https://owasp.org/www-project-rust-security/)
- [Crypto Security Standards](https://crypto.standards.org/)

## 🔄 การปรับปรุงอย่างสม่ำเสมอ

### Monthly Security Tasks

1. **Review Security Logs**: วิเคราะห์ trends และ patterns
2. **Update Dependencies**: Apply security patches
3. **Security Training**: Team awareness และ best practices
4. **Incident Response Drill**: Test response procedures

### Quarterly Reviews

1. **Security Assessment**: Comprehensive security audit
2. **Penetration Testing**: External security testing
3. **Policy Review**: Update security policies
4. **Tool Evaluation**: Assess new security tools

---

**สำคัญ**: Security is an ongoing process, not a one-time setup. Regular monitoring, updates, and improvements are essential to maintain security effectiveness.

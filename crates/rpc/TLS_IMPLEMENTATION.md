# TLS/HTTPS Implementation Plan

## Current Status ✅

**Good news**: BitQuan already has TLS infrastructure!

```rust
// ✅ Already implemented:
- rustls 0.23 (TLS 1.3 only)
- Certificate loading (PEM format)
- Self-signed cert generation (dev mode)
- TlsConfig wrapper
- force_tls flag in RpcConfig
```

## What's Missing ❌

1. **Production certificate management**
   - No Let's Encrypt / ACME integration
   - No automatic renewal
   - No cert validation checking

2. **HTTPS-only enforcement**
   - `force_tls` exists but not mandatory on mainnet
   - No HTTP → HTTPS redirect
   - No HSTS headers

3. **TLS best practices**
   - No cipher suite hardening
   - No cert pinning
   - No OCSP stapling
   - No session resumption optimization

4. **Certificate monitoring**
   - No expiration warnings
   - No health checks
   - No metrics

## Implementation Plan (1-2 weeks)

### Week 1: Core TLS Enforcement

#### Day 1-2: Mandatory TLS on Mainnet
```rust
// crates/rpc/src/lib.rs
pub struct RpcConfig {
    pub require_tls: bool,
    pub tls_cert_path: Option<PathBuf>,
    pub tls_key_path: Option<PathBuf>,
    pub allow_self_signed: bool, // devnet only
}

// Enforce on mainnet
impl RpcConfig {
    pub fn mainnet_default() -> Self {
        Self {
            require_tls: true,  // ✅ Mandatory!
            allow_self_signed: false,
            ..Default::default()
        }
    }
}
```

#### Day 3-4: HTTP → HTTPS Redirect
```rust
// Detect non-TLS connections and reject with upgrade message
fn handle_connection() {
    if force_tls && tls.is_none() {
        send_upgrade_required_response(stream);
        return;
    }
}
```

#### Day 5-7: Security Headers + HSTS
```rust
// Add security headers to all responses
fn add_security_headers(headers: &mut HeaderMap) {
    headers.insert("Strict-Transport-Security",
                   "max-age=31536000; includeSubDomains");
    headers.insert("X-Content-Type-Options", "nosniff");
    headers.insert("X-Frame-Options", "DENY");
    headers.insert("X-XSS-Protection", "1; mode=block");
}
```

### Week 2: Production Features

#### Day 1-3: Certificate Validation
```rust
// Warn if cert expires soon
fn check_cert_expiration(cert: &Certificate) -> Result<()> {
    let expires_in = cert.not_after() - now();
    if expires_in < Duration::from_days(30) {
        warn!("⚠️  Certificate expires in {} days", expires_in.as_days());
    }
    Ok(())
}
```

#### Day 4-5: Let's Encrypt Integration (Optional)
```rust
// Use acme-lib or similar for auto cert renewal
#[cfg(feature = "acme")]
pub fn setup_acme(domain: &str) -> Result<TlsConfig> {
    // TODO: Implement ACME protocol
}
```

#### Day 6-7: Documentation + Testing
- Update README with TLS setup instructions
- Add integration tests
- Performance benchmarks (TLS overhead)
- Security audit checklist

## Priority Features

### P0: Must Have (Week 1)
- [x] ✅ TLS 1.3 only (already done)
- [ ] Mandatory TLS on mainnet
- [ ] HTTP → HTTPS upgrade response
- [ ] HSTS headers
- [ ] Self-signed cert blocked on mainnet

### P1: Should Have (Week 2)
- [ ] Certificate expiration warnings
- [ ] Cipher suite hardening
- [ ] TLS health check endpoint
- [ ] Metrics (handshake time, errors)

### P2: Nice to Have (Future)
- [ ] ACME/Let's Encrypt integration
- [ ] OCSP stapling
- [ ] Certificate pinning
- [ ] Session resumption optimization
- [ ] TLS 1.2 fallback (if needed)

## Testing Strategy

### Unit Tests
```rust
#[test]
fn test_tls_required_on_mainnet() {
    let config = RpcConfig::mainnet_default();
    assert!(config.require_tls);
    assert!(!config.allow_self_signed);
}

#[test]
fn test_self_signed_rejected_on_mainnet() {
    let server = RpcServer::new(handler, "127.0.0.1:8332")
        .with_tls_config(TlsConfig::self_signed()?)
        .require_tls(true);

    assert!(server.validate_for_mainnet().is_err());
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_http_rejected_when_tls_required() {
    let server = spawn_server_with_tls_required();

    let response = reqwest::get("http://localhost:8332/health").await;
    assert!(response.is_err() || response.status() == 426); // Upgrade Required
}

#[tokio::test]
async fn test_https_works() {
    let server = spawn_server_with_tls();

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // test cert
        .build()?;

    let response = client.get("https://localhost:8332/health").await?;
    assert_eq!(response.status(), 200);
}
```

## Security Considerations

### Certificate Management
1. **Development**:
   - Self-signed certs OK
   - Warn users about trust issues

2. **Production**:
   - Use proper CA-signed certs
   - Let's Encrypt recommended (free)
   - Document cert installation

### Cipher Suites (TLS 1.3)
```rust
// rustls 0.23 with TLS 1.3 uses secure defaults:
// - TLS_AES_256_GCM_SHA384
// - TLS_AES_128_GCM_SHA256
// - TLS_CHACHA20_POLY1305_SHA256
// ✅ All quantum-resistant against passive attacks
```

### Common Pitfalls
- ❌ Don't store private keys in git
- ❌ Don't use same cert for dev + prod
- ✅ Use 600 permissions on key files
- ✅ Rotate certs before expiration
- ✅ Monitor cert validity daily

## CLI Integration

### Updated Commands
```bash
# Dev mode (self-signed OK)
bitquan-node --network devnet --rpc-addr 127.0.0.1:18332

# Mainnet (TLS required)
bitquan-node --network mainnet \
  --rpc-addr 0.0.0.0:8332 \
  --tls-cert /etc/bitquan/cert.pem \
  --tls-key /etc/bitquan/key.pem

# Generate self-signed (dev only)
bitquan-node generate-tls-cert --output ./tls/
```

### Configuration File
```toml
# bitquan.toml
[rpc]
bind = "0.0.0.0:8332"
require_tls = true
tls_cert = "/etc/bitquan/cert.pem"
tls_key = "/etc/bitquan/key.pem"
allow_self_signed = false  # mainnet must be false

[rpc.security]
hsts_max_age = 31536000  # 1 year
hsts_include_subdomains = true
```

## Deployment Checklist

### Before Mainnet Launch
- [ ] Valid CA-signed certificate installed
- [ ] Private key permissions: 600 (owner read/write only)
- [ ] Certificate expiration > 30 days
- [ ] HSTS enabled with 1 year max-age
- [ ] Self-signed certs disabled in config
- [ ] TLS health check passing
- [ ] Monitoring set up for cert expiration
- [ ] Backup cert + key stored securely offline
- [ ] Tested with actual RPC clients
- [ ] Documentation updated

### Post-Launch Monitoring
- [ ] Daily cert expiration check
- [ ] TLS handshake success rate > 99%
- [ ] Average handshake time < 50ms
- [ ] No cipher suite downgrade attempts
- [ ] Weekly security scan (nmap, testssl.sh)

## Resources

- [Mozilla SSL Configuration Generator](https://ssl-config.mozilla.org/)
- [Let's Encrypt Documentation](https://letsencrypt.org/docs/)
- [OWASP TLS Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Transport_Layer_Protection_Cheat_Sheet.html)
- [testssl.sh](https://testssl.sh/) - TLS scanner
- [Qualys SSL Labs](https://www.ssllabs.com/ssltest/) - Online TLS tester

---

**Status**: Ready to implement! 🚀
**Estimated Time**: 7-10 days with AI assistance
**Blockers**: None (infrastructure already exists)

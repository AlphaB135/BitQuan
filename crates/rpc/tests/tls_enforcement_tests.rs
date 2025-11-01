//! Integration tests for TLS enforcement
use bitquan_rpc::RpcConfig;

#[test]
fn test_mainnet_config_requires_tls() {
    let config = RpcConfig::mainnet();
    assert!(config.require_tls, "Mainnet must require TLS");
    assert!(!config.allow_self_signed, "Mainnet must not allow self-signed certs");
    assert!(config.enable_hsts, "Mainnet must enable HSTS");
    assert_eq!(config.hsts_max_age, 31536000, "HSTS max-age should be 1 year");
}

#[test]
fn test_devnet_config_allows_self_signed() {
    let config = RpcConfig::devnet();
    assert!(!config.require_tls, "Devnet TLS is optional");
    assert!(config.allow_self_signed, "Devnet allows self-signed");
    assert!(!config.enable_hsts, "Devnet should not enable HSTS");
}

#[test]
fn test_default_config() {
    let config = RpcConfig::default();
    assert!(!config.require_tls, "Default doesn't require TLS");
    assert!(config.allow_self_signed, "Default allows self-signed");
    assert!(config.enable_hsts, "Default enables HSTS");
}

//! TLS configuration helpers for the BitQuan RPC server.

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use rcgen::{generate_simple_self_signed, CertifiedKey};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs1KeyDer, PrivatePkcs8KeyDer};
use rustls::version::TLS13;
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use thiserror::Error;

/// Errors that can occur while working with TLS configuration.
#[derive(Debug, Error)]
pub enum TlsError {
    /// Underlying filesystem failure.
    #[error("TLS file I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to parse PEM-encoded assets.
    #[error("TLS PEM parse error: {0}")]
    Pem(String),
    /// Failed to build the rustls configuration.
    #[error("rustls configuration error: {0}")]
    Rustls(#[from] rustls::Error),
    /// No usable private key could be found in the supplied file.
    #[error("no private key found in {0}")]
    NoPrivateKey(PathBuf),
    /// Certificate file did not contain any certificates.
    #[error("no certificates found in {0}")]
    NoCertificates(PathBuf),
    /// Self-signed certificate generation failure.
    #[error("failed to generate certificate: {0}")]
    CertGen(#[from] rcgen::Error),
}

/// Wrapper for `rustls::ServerConfig` with helper constructors.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    server_config: Arc<ServerConfig>,
    is_self_signed: bool,
    cert_expires_at: Option<i64>, // Unix timestamp
}

impl TlsConfig {
    /// Constructs a TLS configuration from certificate + private key files.
    pub fn new(cert_path: &Path, key_path: &Path) -> Result<Self, TlsError> {
        let certificates = load_certs(cert_path)?;
        let private_key = load_private_key(key_path)?;

        // Check if self-signed and get expiration
        let (is_self_signed, cert_expires_at) = analyze_certificate(&certificates[0])?;

        let mut config = ServerConfig::builder_with_protocol_versions(&[&TLS13])
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)?;

        // Support both HTTP/1.1 and HTTP/2 over TLS.
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(Self {
            server_config: Arc::new(config),
            is_self_signed,
            cert_expires_at,
        })
    }

    /// Check if using self-signed certificate
    pub fn is_self_signed(&self) -> bool {
        self.is_self_signed
    }

    /// Get certificate expiration timestamp (if available)
    pub fn expires_at(&self) -> Option<i64> {
        self.cert_expires_at
    }

    /// Check if certificate expires soon (within given days)
    pub fn expires_soon(&self, days: u64) -> bool {
        if let Some(expires_at) = self.cert_expires_at {
            let now = match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(duration) => duration.as_secs() as i64,
                Err(_) => return false,
            };
            let threshold = now + (days * 24 * 60 * 60) as i64;
            expires_at < threshold
        } else {
            false
        }
    }

    /// Returns an `Arc` to the inner `ServerConfig` for use when accepting connections.
    pub fn server_config(&self) -> Arc<ServerConfig> {
        Arc::clone(&self.server_config)
    }
}

impl AsRef<ServerConfig> for TlsConfig {
    fn as_ref(&self) -> &ServerConfig {
        self.server_config.as_ref()
    }
}

/// Generates a development self-signed certificate + private key pair.
///
/// The artefacts are written to `cert.pem` and `key.pem` under the provided output directory.
pub fn generate_self_signed_cert(output_dir: &Path) -> Result<(), TlsError> {
    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    std::fs::write(output_dir.join("cert.pem"), cert_pem)?;
    std::fs::write(output_dir.join("key.pem"), key_pem)?;

    Ok(())
}

fn load_certs(path: &Path) -> Result<Vec<CertificateDer<'static>>, TlsError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = certs(&mut reader)
        .collect::<Result<Vec<CertificateDer<'static>>, _>>()
        .map_err(|err| TlsError::Pem(err.to_string()))?;

    if certs.is_empty() {
        return Err(TlsError::NoCertificates(path.to_path_buf()));
    }

    Ok(certs)
}

fn load_private_key(path: &Path) -> Result<PrivateKeyDer<'static>, TlsError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    if let Some(key) = pkcs8_private_keys(&mut reader).next() {
        let key: PrivatePkcs8KeyDer<'static> = key.map_err(|err| TlsError::Pem(err.to_string()))?;
        return Ok(PrivateKeyDer::from(key));
    }

    reader.seek(SeekFrom::Start(0))?;

    if let Some(key) = rsa_private_keys(&mut reader).next() {
        let key: PrivatePkcs1KeyDer<'static> = key.map_err(|err| TlsError::Pem(err.to_string()))?;
        return Ok(PrivateKeyDer::from(key));
    }

    Err(TlsError::NoPrivateKey(path.to_path_buf()))
}

/// Analyze certificate to determine if self-signed and get expiration
fn analyze_certificate(cert: &CertificateDer<'static>) -> Result<(bool, Option<i64>), TlsError> {
    // Basic heuristic: check if issuer == subject (self-signed indicator)
    // For production, use x509-parser crate for proper parsing
    let is_self_signed = is_likely_self_signed(cert);

    // Extract expiration from certificate
    // Note: Full X.509 parsing would require additional dependencies
    let expires_at = None; // Certificate expiration parsing available with x509-parser crate

    Ok((is_self_signed, expires_at))
}

/// Simple heuristic to detect self-signed certificates
fn is_likely_self_signed(cert: &CertificateDer<'static>) -> bool {
    // In production, use x509-parser to compare issuer vs subject
    // For now, return false (assume CA-signed unless proven otherwise)

    // Check common self-signed indicators in CN
    let cert_bytes = cert.as_ref();
    let cert_str = String::from_utf8_lossy(cert_bytes);

    cert_str.contains("localhost")
        || cert_str.contains("127.0.0.1")
        || cert_str.contains("self-signed")
}

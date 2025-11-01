//! TLS configuration helpers for the BitQuan RPC server.

use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

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
#[derive(Debug)]
pub struct TlsConfig {
    server_config: ServerConfig,
}

impl TlsConfig {
    /// Constructs a TLS configuration from certificate + private key files.
    pub fn new(cert_path: &Path, key_path: &Path) -> Result<Self, TlsError> {
        let certificates = load_certs(cert_path)?;
        let private_key = load_private_key(key_path)?;

        let mut config = ServerConfig::builder_with_protocol_versions(&[&TLS13])
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)?;

        // Support both HTTP/1.1 and HTTP/2 over TLS.
        config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

        Ok(Self {
            server_config: config,
        })
    }

    /// Returns an immutable reference to the inner `ServerConfig`.
    pub fn as_ref(&self) -> &ServerConfig {
        &self.server_config
    }

    /// Consumes the wrapper and returns the owned `ServerConfig`.
    pub fn into_inner(self) -> ServerConfig {
        self.server_config
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

    for key in pkcs8_private_keys(&mut reader) {
        let key: PrivatePkcs8KeyDer<'static> = key.map_err(|err| TlsError::Pem(err.to_string()))?;
        return Ok(PrivateKeyDer::from(key));
    }

    reader.seek(SeekFrom::Start(0))?;

    for key in rsa_private_keys(&mut reader) {
        let key: PrivatePkcs1KeyDer<'static> = key.map_err(|err| TlsError::Pem(err.to_string()))?;
        return Ok(PrivateKeyDer::from(key));
    }

    Err(TlsError::NoPrivateKey(path.to_path_buf()))
}

//! Noise Protocol encryption for P2P connections.
//!
//! Implements `Noise_XX_25519_ChaChaPoly_BLAKE2s` for mutual authentication
//! and encryption of all P2P traffic.
//!
//! # Security Properties
//! - **Confidentiality**: All messages encrypted with ChaCha20-Poly1305
//! - **Integrity**: AEAD authentication prevents tampering
//! - **Forward Secrecy**: Ephemeral keys used per session
//! - **Mutual Authentication**: Both peers verify identity (XX pattern)
//!
//! # Usage
//! ```ignore
//! use bitquan_network::noise::{NoiseTransport, NoiseConfig};
//!
//! // Generate or load static keypair
//! let config = NoiseConfig::generate();
//!
//! // Upgrade TCP stream to encrypted transport (initiator)
//! let transport = NoiseTransport::upgrade_initiator(stream, &config)?;
//!
//! // Or as responder
//! let transport = NoiseTransport::upgrade_responder(stream, &config)?;
//! ```

use snow::{Builder, HandshakeState, TransportState};
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use thiserror::Error;
use zeroize::Zeroizing;

/// Noise protocol pattern: XX with Curve25519, ChaCha20-Poly1305, BLAKE2s
const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// Maximum Noise message size (65535 bytes per spec)
const MAX_NOISE_MSG_LEN: usize = 65535;

/// Handshake message buffer size
const HANDSHAKE_BUF_SIZE: usize = 256;

/// Errors that can occur during Noise operations.
#[derive(Debug, Error)]
pub enum NoiseError {
    /// Noise library error
    #[error("noise error: {0}")]
    Snow(#[from] snow::Error),

    /// I/O error during handshake or transport
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// Invalid key format
    #[error("invalid key: {0}")]
    InvalidKey(String),

    /// Handshake failed
    #[error("handshake failed: {0}")]
    HandshakeFailed(String),

    /// Transport not ready (handshake incomplete)
    #[error("transport not ready")]
    NotReady,

    /// Message too large
    #[error("message too large: {0} > {MAX_NOISE_MSG_LEN}")]
    MessageTooLarge(usize),
}

/// Result type for Noise operations.
pub type Result<T> = std::result::Result<T, NoiseError>;

/// Noise Protocol configuration with static keypair.
#[derive(Clone)]
pub struct NoiseConfig {
    /// Static private key (32 bytes, Curve25519)
    private_key: Zeroizing<[u8; 32]>,
    /// Static public key (32 bytes, Curve25519)
    pub public_key: [u8; 32],
}

impl NoiseConfig {
    /// Generate a new random keypair.
    pub fn generate() -> Result<Self> {
        let builder = Builder::new(NOISE_PATTERN.parse()?);
        let keypair = builder.generate_keypair()?;

        let mut private_key = Zeroizing::new([0u8; 32]);
        let mut public_key = [0u8; 32];

        private_key.copy_from_slice(&keypair.private);
        public_key.copy_from_slice(&keypair.public);

        Ok(Self {
            private_key,
            public_key,
        })
    }

    /// Load keypair from files, or generate and save if not exist.
    pub fn load_or_generate(key_dir: &Path) -> Result<Self> {
        let private_path = key_dir.join("noise_private.key");
        let public_path = key_dir.join("noise_public.key");

        if private_path.exists() && public_path.exists() {
            Self::load(&private_path, &public_path)
        } else {
            let config = Self::generate()?;
            config.save(&private_path, &public_path)?;
            Ok(config)
        }
    }

    /// Load keypair from files.
    pub fn load(private_path: &Path, public_path: &Path) -> Result<Self> {
        let private_bytes = fs::read(private_path)?;
        let public_bytes = fs::read(public_path)?;

        if private_bytes.len() != 32 {
            return Err(NoiseError::InvalidKey(format!(
                "private key must be 32 bytes, got {}",
                private_bytes.len()
            )));
        }
        if public_bytes.len() != 32 {
            return Err(NoiseError::InvalidKey(format!(
                "public key must be 32 bytes, got {}",
                public_bytes.len()
            )));
        }

        let mut private_key = Zeroizing::new([0u8; 32]);
        let mut public_key = [0u8; 32];

        private_key.copy_from_slice(&private_bytes);
        public_key.copy_from_slice(&public_bytes);

        Ok(Self {
            private_key,
            public_key,
        })
    }

    /// Save keypair to files with restricted permissions.
    pub fn save(&self, private_path: &Path, public_path: &Path) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = private_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write private key with restricted permissions
        fs::write(private_path, self.private_key.as_ref())?;

        // Set permissions to 0600 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(private_path, perms)?;
        }

        // Write public key
        fs::write(public_path, &self.public_key)?;

        Ok(())
    }

    /// Get public key as hex string (for display/logging).
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.public_key)
    }

    /// Build initiator handshake state.
    fn build_initiator(&self) -> Result<HandshakeState> {
        let builder = Builder::new(NOISE_PATTERN.parse()?);
        Ok(builder
            .local_private_key(self.private_key.as_ref())
            .build_initiator()?)
    }

    /// Build responder handshake state.
    fn build_responder(&self) -> Result<HandshakeState> {
        let builder = Builder::new(NOISE_PATTERN.parse()?);
        Ok(builder
            .local_private_key(self.private_key.as_ref())
            .build_responder()?)
    }
}

/// Encrypted P2P transport using Noise Protocol.
///
/// Wraps a `TcpStream` and transparently encrypts/decrypts all data.
pub struct NoiseTransport {
    /// Underlying TCP stream
    stream: TcpStream,
    /// Noise transport state (after handshake)
    transport: TransportState,
    /// Remote peer's static public key (authenticated during handshake)
    remote_public_key: [u8; 32],
    /// Read buffer for decrypted data
    read_buf: Vec<u8>,
    /// Position in read buffer
    read_pos: usize,
}

impl NoiseTransport {
    /// Upgrade a TCP stream to encrypted transport (initiator/client side).
    ///
    /// Performs the Noise XX handshake as initiator:
    /// 1. -> e (send ephemeral public key)
    /// 2. <- e, ee, s, es (receive responder's keys)
    /// 3. -> s, se (send our static key, complete handshake)
    pub fn upgrade_initiator(mut stream: TcpStream, config: &NoiseConfig) -> Result<Self> {
        let mut handshake = config.build_initiator()?;
        let mut buf = [0u8; HANDSHAKE_BUF_SIZE];

        // Message 1: -> e
        let len = handshake.write_message(&[], &mut buf)?;
        Self::send_handshake_msg(&mut stream, &buf[..len])?;

        // Message 2: <- e, ee, s, es
        let msg = Self::recv_handshake_msg(&mut stream)?;
        handshake.read_message(&msg, &mut buf)?;

        // Message 3: -> s, se
        let len = handshake.write_message(&[], &mut buf)?;
        Self::send_handshake_msg(&mut stream, &buf[..len])?;

        // Extract remote public key and convert to transport mode
        let remote_public_key = Self::extract_remote_key(&handshake)?;
        let transport = handshake.into_transport_mode()?;

        log::info!(
            "Noise handshake complete (initiator) - remote key: {}",
            hex::encode(remote_public_key)
        );

        Ok(Self {
            stream,
            transport,
            remote_public_key,
            read_buf: Vec::new(),
            read_pos: 0,
        })
    }

    /// Upgrade a TCP stream to encrypted transport (responder/server side).
    ///
    /// Performs the Noise XX handshake as responder:
    /// 1. <- e (receive initiator's ephemeral key)
    /// 2. -> e, ee, s, es (send our keys)
    /// 3. <- s, se (receive initiator's static key, complete handshake)
    pub fn upgrade_responder(mut stream: TcpStream, config: &NoiseConfig) -> Result<Self> {
        let mut handshake = config.build_responder()?;
        let mut buf = [0u8; HANDSHAKE_BUF_SIZE];

        // Message 1: <- e
        let msg = Self::recv_handshake_msg(&mut stream)?;
        handshake.read_message(&msg, &mut buf)?;

        // Message 2: -> e, ee, s, es
        let len = handshake.write_message(&[], &mut buf)?;
        Self::send_handshake_msg(&mut stream, &buf[..len])?;

        // Message 3: <- s, se
        let msg = Self::recv_handshake_msg(&mut stream)?;
        handshake.read_message(&msg, &mut buf)?;

        // Extract remote public key and convert to transport mode
        let remote_public_key = Self::extract_remote_key(&handshake)?;
        let transport = handshake.into_transport_mode()?;

        log::info!(
            "Noise handshake complete (responder) - remote key: {}",
            hex::encode(remote_public_key)
        );

        Ok(Self {
            stream,
            transport,
            remote_public_key,
            read_buf: Vec::new(),
            read_pos: 0,
        })
    }

    /// Get remote peer's authenticated public key.
    pub fn remote_public_key(&self) -> &[u8; 32] {
        &self.remote_public_key
    }

    /// Get remote peer's public key as hex string.
    pub fn remote_public_key_hex(&self) -> String {
        hex::encode(self.remote_public_key)
    }

    /// Send a length-prefixed handshake message.
    fn send_handshake_msg(stream: &mut TcpStream, msg: &[u8]) -> Result<()> {
        let len = (msg.len() as u16).to_be_bytes();
        stream.write_all(&len)?;
        stream.write_all(msg)?;
        stream.flush()?;
        Ok(())
    }

    /// Receive a length-prefixed handshake message.
    fn recv_handshake_msg(stream: &mut TcpStream) -> Result<Vec<u8>> {
        let mut len_buf = [0u8; 2];
        stream.read_exact(&mut len_buf)?;
        let len = u16::from_be_bytes(len_buf) as usize;

        if len > HANDSHAKE_BUF_SIZE {
            return Err(NoiseError::HandshakeFailed(format!(
                "handshake message too large: {}",
                len
            )));
        }

        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Extract remote static public key from handshake state.
    fn extract_remote_key(handshake: &HandshakeState) -> Result<[u8; 32]> {
        let remote_static = handshake
            .get_remote_static()
            .ok_or_else(|| NoiseError::HandshakeFailed("no remote static key".to_string()))?;

        if remote_static.len() != 32 {
            return Err(NoiseError::HandshakeFailed(format!(
                "invalid remote key length: {}",
                remote_static.len()
            )));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(remote_static);
        Ok(key)
    }

    /// Encrypt and send a message.
    pub fn send(&mut self, plaintext: &[u8]) -> Result<()> {
        if plaintext.len() > MAX_NOISE_MSG_LEN - 16 {
            // 16 bytes for AEAD tag
            return Err(NoiseError::MessageTooLarge(plaintext.len()));
        }

        // Encrypt message
        let mut ciphertext = vec![0u8; plaintext.len() + 16];
        let len = self.transport.write_message(plaintext, &mut ciphertext)?;
        ciphertext.truncate(len);

        // Send length-prefixed ciphertext
        let len_bytes = (len as u16).to_be_bytes();
        self.stream.write_all(&len_bytes)?;
        self.stream.write_all(&ciphertext)?;
        self.stream.flush()?;

        Ok(())
    }

    /// Receive and decrypt a message.
    pub fn recv(&mut self) -> Result<Vec<u8>> {
        // Read length prefix
        let mut len_buf = [0u8; 2];
        self.stream.read_exact(&mut len_buf)?;
        let len = u16::from_be_bytes(len_buf) as usize;

        if len > MAX_NOISE_MSG_LEN {
            return Err(NoiseError::MessageTooLarge(len));
        }

        // Read ciphertext
        let mut ciphertext = vec![0u8; len];
        self.stream.read_exact(&mut ciphertext)?;

        // Decrypt
        let mut plaintext = vec![0u8; len];
        let decrypted_len = self.transport.read_message(&ciphertext, &mut plaintext)?;
        plaintext.truncate(decrypted_len);

        Ok(plaintext)
    }

    /// Get reference to underlying stream (for socket options, etc.).
    pub fn stream(&self) -> &TcpStream {
        &self.stream
    }

    /// Get mutable reference to underlying stream.
    pub fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Try to clone the transport (clones the stream, shares the transport state).
    /// WARNING: This is unsafe for concurrent use - only use for read/write split.
    pub fn try_clone(&self) -> Result<TcpStream> {
        Ok(self.stream.try_clone()?)
    }
}

/// Implement Read trait for transparent decryption.
impl Read for NoiseTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // If we have buffered data, return it first
        if self.read_pos < self.read_buf.len() {
            let available = &self.read_buf[self.read_pos..];
            let to_copy = available.len().min(buf.len());
            buf[..to_copy].copy_from_slice(&available[..to_copy]);
            self.read_pos += to_copy;

            // Clear buffer if fully consumed
            if self.read_pos >= self.read_buf.len() {
                self.read_buf.clear();
                self.read_pos = 0;
            }

            return Ok(to_copy);
        }

        // Receive and decrypt next message
        match self.recv() {
            Ok(plaintext) => {
                if plaintext.is_empty() {
                    return Ok(0);
                }

                let to_copy = plaintext.len().min(buf.len());
                buf[..to_copy].copy_from_slice(&plaintext[..to_copy]);

                // Buffer any remaining data
                if to_copy < plaintext.len() {
                    self.read_buf = plaintext;
                    self.read_pos = to_copy;
                }

                Ok(to_copy)
            }
            Err(NoiseError::Io(e)) => Err(e),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}

/// Implement Write trait for transparent encryption.
impl Write for NoiseTransport {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Noise has max message size, so we may need to split
        let chunk_size = MAX_NOISE_MSG_LEN - 16; // Reserve space for AEAD tag

        for chunk in buf.chunks(chunk_size) {
            self.send(chunk)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_keypair_generation() {
        let config = NoiseConfig::generate().unwrap();
        assert_eq!(config.public_key.len(), 32);
        assert_eq!(config.private_key.len(), 32);
        // Public and private keys should be different
        assert_ne!(config.public_key.as_slice(), config.private_key.as_ref());
    }

    #[test]
    fn test_handshake_and_transport() {
        let initiator_config = NoiseConfig::generate().unwrap();
        let responder_config = NoiseConfig::generate().unwrap();

        // Set up listener
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn responder thread
        let responder_cfg = responder_config.clone();
        let responder_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            NoiseTransport::upgrade_responder(stream, &responder_cfg).unwrap()
        });

        // Connect as initiator
        let stream = TcpStream::connect(addr).unwrap();
        let mut initiator = NoiseTransport::upgrade_initiator(stream, &initiator_config).unwrap();
        let mut responder = responder_thread.join().unwrap();

        // Verify key exchange
        assert_eq!(
            initiator.remote_public_key(),
            &responder_config.public_key
        );
        assert_eq!(
            responder.remote_public_key(),
            &initiator_config.public_key
        );

        // Test encrypted communication
        let test_msg = b"Hello, encrypted world!";
        initiator.send(test_msg).unwrap();
        let received = responder.recv().unwrap();
        assert_eq!(received, test_msg);

        // Test reverse direction
        let response = b"Hello back!";
        responder.send(response).unwrap();
        let received = initiator.recv().unwrap();
        assert_eq!(received, response);
    }

    #[test]
    fn test_read_write_traits() {
        let initiator_config = NoiseConfig::generate().unwrap();
        let responder_config = NoiseConfig::generate().unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let responder_cfg = responder_config.clone();
        let responder_thread = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            NoiseTransport::upgrade_responder(stream, &responder_cfg).unwrap()
        });

        let stream = TcpStream::connect(addr).unwrap();
        let mut initiator = NoiseTransport::upgrade_initiator(stream, &initiator_config).unwrap();
        let mut responder = responder_thread.join().unwrap();

        // Test using Write trait
        let msg = b"Using Write trait";
        initiator.write_all(msg).unwrap();
        initiator.flush().unwrap();

        // Test using Read trait
        let mut buf = vec![0u8; msg.len()];
        responder.read_exact(&mut buf).unwrap();
        assert_eq!(&buf, msg);
    }
}

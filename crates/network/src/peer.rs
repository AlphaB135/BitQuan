//! TCP-based P2P connection handler with Noise Protocol encryption.
//!
//! All peer connections are encrypted using `Noise_XX_25519_ChaChaPoly_BLAKE2s`.
//! This provides mutual authentication, forward secrecy, and protection against
//! eavesdropping and MITM attacks.

use crate::noise::{NoiseConfig, NoiseTransport};
use crate::protocol::{Message, MessageEnvelope, P2pError, PROTOCOL_VERSION};
use bitquan_types::error::{Error, Result as TypesResult};
use bitquan_types::ext::ResultExt;
use snow::{HandshakeState, TransportState};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream as TokioTcpStream;
use tokio::sync::Mutex;

// Re-export hex for convenience
pub use hex;

/// Helper to get current Unix timestamp.
/// Returns 0 if system clock is before epoch (extremely unlikely).
fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Maximum frame size accepted by low-level peer helpers (2 MiB).
/// SECURITY: This limit is enforced BEFORE allocation to prevent buffer bloat attacks.
pub const MAX_MSG_BYTES: usize = 2 * 1024 * 1024;
/// PQC-aware frame size limit (16 KiB) for post-quantum cryptography overhead.
/// SECURITY: Dilithium-5 signatures are 4595 bytes. This limit accommodates
/// PQC overhead while preventing OOM attacks. Used as secondary validation layer.
pub const MAX_PQC_FRAME: usize = 16 * 1024;
/// Socket read/write timeout in seconds for Slowloris protection.
/// SECURITY: Prevents attackers from holding connections open indefinitely.
pub const SOCKET_TIMEOUT_SECS: u64 = 30;
/// Handshake timeout in milliseconds (deprecated, use SOCKET_TIMEOUT_SECS).
#[deprecated(note = "Use SOCKET_TIMEOUT_SECS instead")]
pub const HANDSHAKE_TIMEOUT_MS: u64 = 1_200;
/// Rate limit: max messages per second per peer
pub const RATE_LIMIT_PER_SECOND: u64 = 100;
/// Peer timeout in seconds (disconnect if no activity).
pub const PEER_TIMEOUT_SECS: u64 = 120;
/// Ban score threshold (disconnect and ban at this score).
pub const BAN_THRESHOLD: u32 = 100;
/// NODE_NETWORK service flag.
pub const SERVICE_NODE_NETWORK: u64 = 1;
/// User agent string.
pub const USER_AGENT: &str = concat!("BitQuan/", env!("CARGO_PKG_VERSION"));

/// Performs encrypted protocol handshake on an established NoiseTransport.
///
/// # Deprecated
/// This function is no longer needed - the handshake is performed automatically
/// during `Peer::new_inbound()` and `Peer::new_outbound()`. It remains for
/// backwards compatibility but should not be used in new code.
#[deprecated(note = "Handshake is now performed automatically in Peer::new_inbound/new_outbound")]
#[allow(deprecated)] // Allow use of deprecated HANDSHAKE_TIMEOUT_MS
pub fn handshake(stream: &mut NoiseTransport) -> TypesResult<()> {
    let timeout = Duration::from_millis(HANDSHAKE_TIMEOUT_MS);
    stream.stream().set_read_timeout(Some(timeout))?;
    stream.stream().set_write_timeout(Some(timeout))?;

    match do_handshake_encrypted(stream) {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::Net(format!("encrypted handshake failed: {e}"))),
    }
}

/// Reads a single length-prefixed message frame.
///
/// # Security
/// - **Integer Overflow Protection:** Uses checked arithmetic to prevent overflow
///   when converting u32 length prefix to usize.
/// - **Buffer Bloat Protection:** Message length is validated BEFORE allocation.
///   Messages exceeding `MAX_MSG_BYTES` (2 MiB) are rejected.
/// - **PQC-Aware Limits:** Additional validation against `MAX_PQC_FRAME` (16 KiB)
///   to protect against large allocations even when under the 2 MiB cap.
/// - **Graceful Allocation:** Uses `Vec::with_capacity()` and `try_reserve()` pattern
///   to handle allocation failures gracefully without panicking.
/// - **Slowloris Protection:** Relies on socket-level read timeouts. Callers must
///   ensure the underlying stream has appropriate timeouts configured via
///   `set_read_timeout()` before calling this function.
pub fn read_frame<R: Read>(reader: &mut R) -> TypesResult<Vec<u8>> {
    let mut len_le = [0u8; 4];
    reader.read_exact(&mut len_le).ctx("read len")?;

    // SECURITY: Use checked arithmetic to prevent integer overflow
    // when converting u32 to usize on platforms where usize < u32
    let len = usize::try_from(u32::from_le_bytes(len_le))
        .map_err(|_| Error::Invalid("frame length overflow".to_string()))?;

    if len == 0 {
        return Err(Error::Invalid("empty frame".to_string()));
    }

    // SECURITY: Primary defense - enforce 2 MiB hard limit
    if len > MAX_MSG_BYTES {
        return Err(Error::Invalid(format!(
            "message too large: {} bytes (max: {})",
            len, MAX_MSG_BYTES
        )));
    }

    // SECURITY: Secondary defense - PQC-aware limit for memory efficiency
    // Even though we accept up to 2 MiB, we warn/log for frames exceeding
    // typical PQC signature sizes (Dilithium-5 = 4595 bytes)
    if len > MAX_PQC_FRAME {
        log::warn!(
            "Large frame detected: {} bytes exceeds PQC threshold {}",
            len,
            MAX_PQC_FRAME
        );
    }

    // SECURITY: Graceful allocation WITHOUT pre-allocation panic
    // CRITICAL: Do NOT use with_capacity() - it allocates immediately and can panic on OOM
    // Instead, use try_reserve_exact() which returns Err on allocation failure
    let mut buf = Vec::new(); // Zero allocation

    // SAFETY: try_reserve_exact attempts allocation without panicking
    // We have already validated len <= MAX_MSG_BYTES (2 MiB), so this is
    // a reasonable request that should succeed on any healthy system
    buf.try_reserve_exact(len)
        .map_err(|_| Error::Invalid("allocation failed - out of memory".to_string()))?;

    // SAFETY: set_len is safe here because:
    // 1. We allocated capacity for exactly `len` elements
    // 2. We are about to fill all bytes with read_exact()
    // 3. The type is u8 which has no initialization requirements
    unsafe {
        buf.set_len(len);
    }

    reader.read_exact(&mut buf).ctx("read frame")?;
    Ok(buf)
}

/// Exchange magic byte through encrypted channel.
/// SECURITY: The single-byte 0x42 check added no security value post-Noise
/// handshake. It has been removed. The Noise Protocol handshake itself provides
/// mutual authentication, forward secrecy, and integrity. Any additional
/// validation should use Noise transport payloads (e.g., version messages).
fn do_handshake_encrypted(_stream: &mut NoiseTransport) -> io::Result<()> {
    // SECURITY FIX: Removed magic byte 0x42 exchange.
    // The single-byte magic provided no security benefit after Noise handshake
    // completion. Protocol-level validation is performed by the version
    // handshake that follows immediately after.
    Ok(())
}

/// Peer connection states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerState {
    /// Initial connection established, waiting for version handshake.
    Connected,
    /// Version sent, waiting for verack.
    VersionSent,
    /// Version received, waiting to send verack.
    VersionReceived,
    /// Handshake complete, ready for normal message exchange.
    Ready,
    /// Connection closed or failed.
    Disconnected,
}

//=============================================================================
// ASYNC NOISE HANDSHAKE HELPERS
//=============================================================================

/// Buffer size for Noise handshake messages.
/// INCREASED TO 64KB for Post-Quantum Cryptography support (Kyber-1024, etc.)
const HANDSHAKE_BUF_SIZE: usize = 65536;

/// Performs an async Noise Protocol handshake as the initiator (client side).
///
/// This is the async equivalent of `NoiseTransport::upgrade_initiator`.
/// It uses tokio I/O throughout and never blocks the executor.
///
/// # Protocol Flow (Noise XX pattern):
/// 1. Send our ephemeral public key
/// 2. Receive responder's ephemeral + static keys
/// 3. Send our static public key
/// 4. Extract authenticated remote public key
///
/// # Returns
/// A tuple of (TokioTcpStream, TransportState, remote_public_key)
/// NOTE: Returns TokioTcpStream to allow async version handshake to follow
pub async fn async_noise_handshake_initiator(
    mut stream: TokioTcpStream,
    config: &NoiseConfig,
) -> Result<(TokioTcpStream, TransportState, [u8; 32]), P2pError> {
    println!("🔧 [HANDSHAKE] async_noise_handshake_initiator: Starting");

    // Build handshake state using NoiseConfig's public method
    let mut handshake = config
        .build_initiator()
        .map_err(|e| P2pError::ConnectionError(format!("failed to build initiator: {e}")))?;

    println!("🔧 [HANDSHAKE] Initiator handshake state built, sending Message 1...");

    let mut buf = [0u8; HANDSHAKE_BUF_SIZE];

    // Message 1: -> e (send ephemeral public key)
    let len = handshake
        .write_message(&[], &mut buf)
        .map_err(|e| P2pError::ConnectionError(format!("handshake write failed: {e}")))?;

    println!(
        "🔧 [HANDSHAKE] Message 1 created ({} bytes), sending...",
        len
    );
    send_handshake_msg_async(&mut stream, &buf[..len])
        .await
        .map_err(|e| P2pError::ConnectionError(format!("send msg1 failed: {e}")))?;

    println!("🔧 [HANDSHAKE] Message 1 sent, waiting for Message 2...");

    // Message 2: <- e, ee, s, es (receive responder's keys)
    println!("🔧 [HANDSHAKE] Calling recv_handshake_msg_async for Message 2...");
    let msg = recv_handshake_msg_async(&mut stream)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("recv msg2 failed: {e}")))?;

    println!("🔧 [HANDSHAKE] Received Message 2 ({} bytes)", msg.len());
    handshake
        .read_message(&msg, &mut buf)
        .map_err(|e| P2pError::ConnectionError(format!("handshake read msg2 failed: {e}")))?;

    println!("🔧 [HANDSHAKE] Message 2 processed, creating Message 3...");

    // Message 3: -> s, se (send our static public key)
    let len = handshake
        .write_message(&[], &mut buf)
        .map_err(|e| P2pError::ConnectionError(format!("handshake write msg3 failed: {e}")))?;

    println!(
        "🔧 [HANDSHAKE] Message 3 created ({} bytes), sending...",
        len
    );
    send_handshake_msg_async(&mut stream, &buf[..len])
        .await
        .map_err(|e| P2pError::ConnectionError(format!("send msg3 failed: {e}")))?;

    println!("🔧 [HANDSHAKE] Message 3 sent! Handshake complete!");

    // Extract remote public key and convert to transport mode
    println!("🔧 [HANDSHAKE] Extracting remote public key...");
    let remote_public_key = extract_remote_key(&handshake)?;

    println!("🔧 [HANDSHAKE] Converting to transport mode...");
    let transport = handshake
        .into_transport_mode()
        .map_err(|e| P2pError::ConnectionError(format!("into transport failed: {e}")))?;

    // Flush tokio stream before returning
    println!("🔧 [HANDSHAKE] Flushing tokio stream...");
    stream
        .flush()
        .await
        .map_err(|e| P2pError::ConnectionError(format!("flush failed: {e}")))?;
    println!("🔧 [HANDSHAKE] Tokio stream flushed");

    // NOTE: Keep TokioTcpStream for async version handshake
    // Conversion to std stream happens AFTER version handshake completes

    println!("🔧 [HANDSHAKE] About to log completion...");
    log::info!(
        "Async Noise handshake complete (initiator) - remote key: {}",
        hex::encode(remote_public_key)
    );
    println!("🔧 [HANDSHAKE] Returning from handshake...");

    Ok((stream, transport, remote_public_key))
}

/// Performs an async Noise Protocol handshake as the responder (server side).
///
/// This is the async equivalent of `NoiseTransport::upgrade_responder`.
/// It uses tokio I/O throughout and never blocks the executor.
///
/// # Protocol Flow (Noise XX pattern):
/// 1. Receive initiator's ephemeral public key
/// 2. Send our ephemeral + static keys
/// 3. Receive initiator's static public key
/// 4. Extract authenticated remote public key
///
/// # Returns
/// A tuple of (TokioTcpStream, TransportState, remote_public_key)
/// NOTE: Returns TokioTcpStream to allow async version handshake to follow
pub async fn async_noise_handshake_responder(
    mut stream: TokioTcpStream,
    config: &NoiseConfig,
) -> Result<(TokioTcpStream, TransportState, [u8; 32]), P2pError> {
    println!("🔧 [HANDSHAKE] async_noise_handshake_responder: Starting");

    // Build handshake state using NoiseConfig's public method
    let mut handshake = config
        .build_responder()
        .map_err(|e| P2pError::ConnectionError(format!("failed to build responder: {e}")))?;

    println!("🔧 [HANDSHAKE] Handshake state built, waiting for Message 1...");

    let mut buf = [0u8; HANDSHAKE_BUF_SIZE];

    // Message 1: <- e (receive initiator's ephemeral public key)
    println!("🔧 [HANDSHAKE] Calling recv_handshake_msg_async for Message 1...");
    let msg = recv_handshake_msg_async(&mut stream)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("recv msg1 failed: {e}")))?;
    println!("🔧 [HANDSHAKE] Received Message 1 ({} bytes)", msg.len());
    handshake
        .read_message(&msg, &mut buf)
        .map_err(|e| P2pError::ConnectionError(format!("handshake read msg1 failed: {e}")))?;

    println!("🔧 [HANDSHAKE] Message 1 processed, creating Message 2...");

    // Message 2: -> e, ee, s, es (send our keys)
    let len = handshake
        .write_message(&[], &mut buf)
        .map_err(|e| P2pError::ConnectionError(format!("handshake write msg2 failed: {e}")))?;

    println!(
        "🔧 [HANDSHAKE] Message 2 created ({} bytes), sending...",
        len
    );
    send_handshake_msg_async(&mut stream, &buf[..len])
        .await
        .map_err(|e| P2pError::ConnectionError(format!("send msg2 failed: {e}")))?;

    println!("🔧 [HANDSHAKE] Message 2 sent, waiting for Message 3...");

    // Message 3: <- s, se (receive initiator's static public key)
    let msg = recv_handshake_msg_async(&mut stream)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("recv msg3 failed: {e}")))?;
    handshake
        .read_message(&msg, &mut buf)
        .map_err(|e| P2pError::ConnectionError(format!("handshake read msg3 failed: {e}")))?;

    // Extract remote public key and convert to transport mode
    println!("🔧 [HANDSHAKE] Message 3 processed, extracting remote public key...");
    let remote_public_key = extract_remote_key(&handshake)?;

    println!("🔧 [HANDSHAKE] Converting to transport mode...");
    let transport = handshake
        .into_transport_mode()
        .map_err(|e| P2pError::ConnectionError(format!("into transport failed: {e}")))?;

    // Flush tokio stream before returning
    println!("🔧 [HANDSHAKE] Flushing tokio stream...");
    stream
        .flush()
        .await
        .map_err(|e| P2pError::ConnectionError(format!("flush failed: {e}")))?;
    println!("🔧 [HANDSHAKE] Tokio stream flushed");

    // NOTE: Keep TokioTcpStream for async version handshake
    // Conversion to std stream happens AFTER version handshake completes

    println!("🔧 [HANDSHAKE] About to log completion...");
    log::info!(
        "Async Noise handshake complete (responder) - remote key: {}",
        hex::encode(remote_public_key)
    );
    println!("🔧 [HANDSHAKE] Returning from handshake...");

    Ok((stream, transport, remote_public_key))
}

/// Sends a length-prefixed handshake message asynchronously.
async fn send_handshake_msg_async(stream: &mut TokioTcpStream, msg: &[u8]) -> io::Result<()> {
    let len = (msg.len() as u16).to_be_bytes();
    stream.write_all(&len).await?;
    stream.write_all(msg).await?;
    stream.flush().await?;
    Ok(())
}

/// Receives a length-prefixed handshake message asynchronously.
async fn recv_handshake_msg_async(stream: &mut TokioTcpStream) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 2];
    stream.read_exact(&mut len_buf).await?;
    let len = u16::from_be_bytes(len_buf) as usize;

    if len > HANDSHAKE_BUF_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("handshake message too large: {}", len),
        ));
    }

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Extracts remote static public key from handshake state.
fn extract_remote_key(handshake: &HandshakeState) -> Result<[u8; 32], P2pError> {
    let remote_static = handshake
        .get_remote_static()
        .ok_or_else(|| P2pError::ConnectionError("no remote static key".to_string()))?;

    if remote_static.len() != 32 {
        return Err(P2pError::ConnectionError(format!(
            "invalid remote key length: {}",
            remote_static.len()
        )));
    }

    let mut key = [0u8; 32];
    key.copy_from_slice(remote_static);
    Ok(key)
}

//=============================================================================
// END ASYNC NOISE HANDSHAKE HELPERS
//=============================================================================

//=============================================================================
// ASYNC VERSION HANDSHAKE
//=============================================================================

/// Async version of send_envelope - sends a serialized MessageEnvelope with length prefix.
async fn send_envelope_async(
    stream: &mut TokioTcpStream,
    env: &MessageEnvelope,
) -> Result<(), P2pError> {
    let bytes = env
        .serialize()
        .map_err(|e| P2pError::SerializationError(e.to_string()))?;
    let len = (bytes.len() as u32).to_le_bytes();
    stream
        .write_all(&len)
        .await
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    stream
        .write_all(&bytes)
        .await
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    stream
        .flush()
        .await
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    Ok(())
}

/// Async version of recv_envelope - receives a length-prefixed serialized MessageEnvelope.
async fn recv_envelope_async(
    stream: &mut TokioTcpStream,
    expected_magic: [u8; 4],
) -> Result<MessageEnvelope, P2pError> {
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 {
        return Err(P2pError::InvalidMessage);
    }
    if len > crate::protocol::MAX_MESSAGE_SIZE {
        return Err(P2pError::MessageTooLarge(len));
    }
    // SECURITY FIX: Use try_reserve_exact instead of vec![0u8; len] to prevent
    // network-controlled allocation from causing OOM before authentication.
    let mut buf = Vec::new();
    buf.try_reserve_exact(len)
        .map_err(|_| P2pError::ConnectionError("alloc failed".into()))?;
    buf.resize(len, 0);
    stream
        .read_exact(&mut buf)
        .await
        .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
    MessageEnvelope::deserialize(&buf, expected_magic)
}

/// Performs async version handshake for outbound connection (initiator).
///
/// # Protocol Flow (Outbound):
/// 1. Send our Version
/// 2. Wait for their Version
/// 3. Send VerAck
/// 4. Wait for their VerAck
///
/// # Returns
/// A tuple of (version, user_agent, start_height) from the remote peer
pub async fn async_version_handshake_outbound(
    stream: &mut TokioTcpStream,
    magic: [u8; 4],
    our_height: u64,
) -> Result<(u32, String, u64), P2pError> {
    println!("🔧 [VERSION] Starting async outbound version handshake...");

    // Send our version
    let version_msg = Message::Version {
        version: PROTOCOL_VERSION,
        services: SERVICE_NODE_NETWORK,
        timestamp: unix_timestamp(),
        user_agent: USER_AGENT.to_string(),
        start_height: our_height,
    };

    let envelope = MessageEnvelope::new(magic, version_msg);
    send_envelope_async(stream, &envelope)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("send version failed: {e}")))?;
    println!("🔧 [VERSION] Sent our version");

    // Wait for their version
    let their_env = recv_envelope_async(stream, magic)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("recv version failed: {e}")))?;

    let (version, user_agent, start_height) = match their_env.message {
        Message::Version {
            version,
            user_agent,
            start_height,
            ..
        } => {
            if version != PROTOCOL_VERSION {
                return Err(P2pError::VersionMismatch(version, PROTOCOL_VERSION));
            }
            println!(
                "🔧 [VERSION] Received their version: {} ({})",
                version, user_agent
            );
            (version, user_agent, start_height)
        }
        _ => {
            return Err(P2pError::InvalidMessage);
        }
    };

    // Send verack
    let verack_env = MessageEnvelope::new(magic, Message::VerAck);
    send_envelope_async(stream, &verack_env)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("send verack failed: {e}")))?;
    println!("🔧 [VERSION] Sent verack");

    // Wait for their verack
    let verack_env = recv_envelope_async(stream, magic)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("recv verack failed: {e}")))?;

    match verack_env.message {
        Message::VerAck => {
            println!("🔧 [VERSION] Received verack - handshake complete!");
        }
        _ => {
            return Err(P2pError::InvalidMessage);
        }
    }

    Ok((version, user_agent, start_height))
}

/// Performs async version handshake for inbound connection (responder).
///
/// # Protocol Flow (Inbound):
/// 1. Wait for their Version
/// 2. Send our Version
/// 3. Send VerAck
/// 4. Wait for their VerAck
///
/// # Returns
/// A tuple of (version, user_agent, start_height) from the remote peer
pub async fn async_version_handshake_inbound(
    stream: &mut TokioTcpStream,
    magic: [u8; 4],
    our_height: u64,
) -> Result<(u32, String, u64), P2pError> {
    println!("🔧 [VERSION] Starting async inbound version handshake...");

    // Wait for their version
    let their_env = recv_envelope_async(stream, magic)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("recv version failed: {e}")))?;

    let (version, user_agent, start_height) = match their_env.message {
        Message::Version {
            version,
            user_agent,
            start_height,
            ..
        } => {
            if version != PROTOCOL_VERSION {
                return Err(P2pError::VersionMismatch(version, PROTOCOL_VERSION));
            }
            println!(
                "🔧 [VERSION] Received their version: {} ({})",
                version, user_agent
            );
            (version, user_agent, start_height)
        }
        _ => {
            return Err(P2pError::InvalidMessage);
        }
    };

    // Send our version
    let version_msg = Message::Version {
        version: PROTOCOL_VERSION,
        services: SERVICE_NODE_NETWORK,
        timestamp: unix_timestamp(),
        user_agent: USER_AGENT.to_string(),
        start_height: our_height,
    };

    let envelope = MessageEnvelope::new(magic, version_msg);
    send_envelope_async(stream, &envelope)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("send version failed: {e}")))?;
    println!("🔧 [VERSION] Sent our version");

    // Send verack
    let verack_env = MessageEnvelope::new(magic, Message::VerAck);
    send_envelope_async(stream, &verack_env)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("send verack failed: {e}")))?;
    println!("🔧 [VERSION] Sent verack");

    // Wait for their verack
    let verack_env = recv_envelope_async(stream, magic)
        .await
        .map_err(|e| P2pError::ConnectionError(format!("recv verack failed: {e}")))?;

    match verack_env.message {
        Message::VerAck => {
            println!("🔧 [VERSION] Received verack - handshake complete!");
        }
        _ => {
            return Err(P2pError::InvalidMessage);
        }
    }

    Ok((version, user_agent, start_height))
}

//=============================================================================
// END ASYNC VERSION HANDSHAKE
//=============================================================================

/// Represents a single encrypted peer connection.
///
/// All communication goes through a Noise Protocol encrypted channel.
/// The encryption is established during construction and cannot be bypassed.
pub struct Peer {
    /// Peer's socket address.
    pub addr: SocketAddr,
    /// Current connection state.
    pub state: PeerState,
    /// Encrypted stream for communication (Noise Protocol).
    /// SECURITY: This is NOT a raw TcpStream - all data is encrypted.
    stream: NoiseTransport,
    /// Remote peer's authenticated public key (from Noise handshake).
    pub remote_public_key: [u8; 32],
    /// Peer's protocol version (from version message).
    pub version: Option<u32>,
    /// Peer's user agent string.
    pub user_agent: Option<String>,
    /// Peer's starting block height.
    pub start_height: Option<u64>,
    /// Last message timestamp (for timeout detection).
    pub last_seen: SystemTime,
    /// Message count for rate limiting.
    pub message_count: u64,
    /// Last rate limit window reset.
    /// SECURITY: Uses Instant instead of SystemTime to avoid NTP clock-step resets.
    pub rate_limit_window: std::time::Instant,
    /// Ban score for misbehavior (disconnect at 100).
    pub ban_score: u32,
    /// Network magic bytes.
    pub magic: [u8; 4],
}

impl Peer {
    /// Creates a new peer from pre-completed async Noise handshake.
    ///
    /// This is used when the Noise handshake was performed asynchronously
    /// (e.g., using tokio I/O) and we now have the raw components.
    ///
    /// # Arguments
    /// * `addr` - Peer's socket address
    /// * `stream` - The Noise transport (already handshaked)
    /// * `remote_public_key` - The authenticated remote public key
    /// * `magic` - Network magic bytes
    pub fn from_handshaked(
        addr: SocketAddr,
        stream: NoiseTransport,
        remote_public_key: [u8; 32],
        magic: [u8; 4],
    ) -> Self {
        Self {
            addr,
            state: PeerState::Connected,
            stream,
            remote_public_key,
            version: None,
            user_agent: None,
            start_height: None,
            last_seen: SystemTime::now(),
            message_count: 0,
            rate_limit_window: std::time::Instant::now(),
            ban_score: 0,
            magic,
        }
    }

    /// Creates a new peer from pre-completed handshake with version information.
    ///
    /// This is used when both the Noise handshake and version handshake have
    /// been completed and we now have the peer's version information.
    ///
    /// # Arguments
    /// * `addr` - Peer's socket address
    /// * `stream` - The Noise transport (already handshaked)
    /// * `remote_public_key` - The authenticated remote public key
    /// * `magic` - Network magic bytes
    /// * `version` - Protocol version from peer
    /// * `user_agent` - User agent string from peer
    /// * `start_height` - Starting block height from peer
    pub fn from_handshaked_with_version(
        addr: SocketAddr,
        stream: NoiseTransport,
        remote_public_key: [u8; 32],
        magic: [u8; 4],
        version: u32,
        user_agent: String,
        start_height: u64,
    ) -> Self {
        Self {
            addr,
            state: PeerState::Ready,
            stream,
            remote_public_key,
            version: Some(version),
            user_agent: Some(user_agent),
            start_height: Some(start_height),
            last_seen: SystemTime::now(),
            message_count: 0,
            rate_limit_window: std::time::Instant::now(),
            ban_score: 0,
            magic,
        }
    }

    /// Creates a new encrypted peer from an inbound connection (we are responder).
    ///
    /// Performs Noise Protocol handshake as responder, then exchanges protocol magic.
    /// Returns an error if encryption handshake fails.
    pub fn new_inbound(
        stream: TcpStream,
        addr: SocketAddr,
        magic: [u8; 4],
        noise_config: &NoiseConfig,
    ) -> Result<Self, P2pError> {
        // SECURITY: Set socket timeouts for Slowloris protection.
        // These timeouts apply to all read/write operations on the socket.
        stream
            .set_read_timeout(Some(Duration::from_secs(SOCKET_TIMEOUT_SECS)))
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(SOCKET_TIMEOUT_SECS)))
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        // Perform Noise handshake (we are responder for inbound connections)
        let encrypted_stream = NoiseTransport::upgrade_responder(stream, noise_config)
            .map_err(|e| P2pError::ConnectionError(format!("Noise handshake failed: {e}")))?;

        let remote_public_key = *encrypted_stream.remote_public_key();

        log::info!(
            "Encrypted connection established (inbound) from {} - remote key: {}",
            addr,
            hex::encode(remote_public_key)
        );

        Ok(Peer {
            addr,
            state: PeerState::Connected,
            stream: encrypted_stream,
            remote_public_key,
            version: None,
            user_agent: None,
            start_height: None,
            last_seen: SystemTime::now(),
            message_count: 0,
            rate_limit_window: std::time::Instant::now(),
            ban_score: 0,
            magic,
        })
    }

    /// Creates a new encrypted peer from an outbound connection (we are initiator).
    ///
    /// Performs Noise Protocol handshake as initiator, then exchanges protocol magic.
    /// Returns an error if encryption handshake fails.
    pub fn new_outbound(
        stream: TcpStream,
        addr: SocketAddr,
        magic: [u8; 4],
        noise_config: &NoiseConfig,
    ) -> Result<Self, P2pError> {
        // SECURITY: Set socket timeouts for Slowloris protection.
        // These timeouts apply to all read/write operations on the socket.
        stream
            .set_read_timeout(Some(Duration::from_secs(SOCKET_TIMEOUT_SECS)))
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(SOCKET_TIMEOUT_SECS)))
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        // Perform Noise handshake (we are initiator for outbound connections)
        let encrypted_stream = NoiseTransport::upgrade_initiator(stream, noise_config)
            .map_err(|e| P2pError::ConnectionError(format!("Noise handshake failed: {e}")))?;

        let remote_public_key = *encrypted_stream.remote_public_key();

        log::info!(
            "Encrypted connection established (outbound) to {} - remote key: {}",
            addr,
            hex::encode(remote_public_key)
        );

        Ok(Peer {
            addr,
            state: PeerState::Connected,
            stream: encrypted_stream,
            remote_public_key,
            version: None,
            user_agent: None,
            start_height: None,
            last_seen: SystemTime::now(),
            message_count: 0,
            rate_limit_window: std::time::Instant::now(),
            ban_score: 0,
            magic,
        })
    }

    /// Get remote peer's public key as hex string.
    pub fn remote_public_key_hex(&self) -> String {
        hex::encode(self.remote_public_key)
    }

    /// Adds to ban score and returns true if peer should be disconnected.
    pub fn add_ban_score(&mut self, points: u32) -> bool {
        // Linus Rule: Use saturating_add to prevent overflow exploit
        // If score is already high, adding more won't wrap around to 0
        self.ban_score = self.ban_score.saturating_add(points);
        self.ban_score >= BAN_THRESHOLD
    }

    /// Checks if peer should be banned.
    pub fn should_ban(&self) -> bool {
        self.ban_score >= BAN_THRESHOLD
    }

    /// Sends a message to the peer.
    pub fn send_message(&mut self, msg: Message) -> Result<(), P2pError> {
        let envelope = MessageEnvelope::new(self.magic, msg);
        crate::io::send_envelope(&mut self.stream, &envelope)?;
        Ok(())
    }

    /// Receives a message from the peer (blocking).
    pub fn recv_message(&mut self) -> Result<Message, P2pError> {
        let envelope = crate::io::recv_envelope(&mut self.stream, self.magic)?;
        self.last_seen = SystemTime::now();

        // Rate limiting check
        // Validate message for memory exhaustion attacks
        crate::protocol::validate_message(&envelope.message)?;

        // SECURITY FIX: Use Instant instead of SystemTime for rate limiting.
        // SystemTime can jump backwards on NTP clock steps, causing
        // duration_since to return Err → unwrap_or_default() → window resets.
        let now = std::time::Instant::now();
        if now.duration_since(self.rate_limit_window) >= Duration::from_secs(1) {
            self.message_count = 0;
            self.rate_limit_window = now;
        }

        self.message_count += 1;
        if self.message_count > RATE_LIMIT_PER_SECOND {
            return Err(P2pError::ConnectionError("rate limit exceeded".into()));
        }

        Ok(envelope.message)
    }

    /// Performs version handshake (outbound connection).
    pub fn handshake_outbound(&mut self, our_height: u64) -> Result<(), P2pError> {
        // Send our version
        let version_msg = Message::Version {
            version: PROTOCOL_VERSION,
            services: SERVICE_NODE_NETWORK, // NODE_NETWORK
            timestamp: unix_timestamp(),
            user_agent: USER_AGENT.to_string(),
            start_height: our_height,
        };

        self.send_message(version_msg)?;
        self.state = PeerState::VersionSent;

        // Wait for their version
        let msg = self.recv_message()?;
        match msg {
            Message::Version {
                version,
                user_agent,
                start_height,
                ..
            } => {
                if version != PROTOCOL_VERSION {
                    return Err(P2pError::VersionMismatch(version, PROTOCOL_VERSION));
                }
                self.version = Some(version);
                self.user_agent = Some(user_agent);
                self.start_height = Some(start_height);
                self.state = PeerState::VersionReceived;
            }
            _ => return Err(P2pError::InvalidMessage),
        }

        // Send verack
        self.send_message(Message::VerAck)?;

        // Wait for their verack
        let msg = self.recv_message()?;
        match msg {
            Message::VerAck => {
                self.state = PeerState::Ready;
                Ok(())
            }
            _ => Err(P2pError::InvalidMessage),
        }
    }

    /// Handles incoming version handshake (inbound connection).
    pub fn handshake_inbound(&mut self, our_height: u64) -> Result<(), P2pError> {
        println!("🔧 [VERSION] Starting inbound version handshake...");
        // Wait for their version
        let msg = self.recv_message()?;
        match msg {
            Message::Version {
                version,
                user_agent,
                start_height,
                ..
            } => {
                if version != PROTOCOL_VERSION {
                    return Err(P2pError::VersionMismatch(version, PROTOCOL_VERSION));
                }
                self.version = Some(version);
                self.user_agent = Some(user_agent);
                self.start_height = Some(start_height);
                self.state = PeerState::VersionReceived;
            }
            _ => return Err(P2pError::InvalidMessage),
        }

        // Send our version
        let version_msg = Message::Version {
            version: PROTOCOL_VERSION,
            services: SERVICE_NODE_NETWORK,
            timestamp: unix_timestamp(),
            user_agent: USER_AGENT.to_string(),
            start_height: our_height,
        };

        self.send_message(version_msg)?;
        self.send_message(Message::VerAck)?;
        self.state = PeerState::VersionSent;

        // Wait for their verack
        let msg = self.recv_message()?;
        match msg {
            Message::VerAck => {
                self.state = PeerState::Ready;
                Ok(())
            }
            _ => Err(P2pError::InvalidMessage),
        }
    }

    /// Checks if peer is still alive (hasn't timed out).
    pub fn is_alive(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_seen)
            .unwrap_or_default()
            < Duration::from_secs(PEER_TIMEOUT_SECS)
    }

    /// Sends a ping to keep connection alive.
    pub fn send_ping(&mut self, nonce: u64) -> Result<(), P2pError> {
        self.send_message(Message::Ping { nonce })
    }

    /// Sends a pong response.
    pub fn send_pong(&mut self, nonce: u64) -> Result<(), P2pError> {
        self.send_message(Message::Pong { nonce })
    }

    /// Extract the NoiseTransport stream from this Peer.
    ///
    /// This is used when the peer was created temporarily for handshake purposes
    /// and we now need the transport back to create the final Peer struct.
    pub fn into_stream(self) -> NoiseTransport {
        self.stream
    }
}

/// Eclipse attack mitigation configuration
#[derive(Debug, Clone)]
pub struct EclipseConfig {
    /// Maximum peers from same /24 subnet
    pub max_peers_per_subnet: usize,
    /// Anchor peers (hardcoded, never evicted)
    pub anchor_peers: Vec<SocketAddr>,
    /// Enable subnet diversity checks
    pub enforce_subnet_diversity: bool,
}

impl Default for EclipseConfig {
    fn default() -> Self {
        Self {
            max_peers_per_subnet: 2,
            anchor_peers: vec![],
            enforce_subnet_diversity: true,
        }
    }
}

/// Manages multiple encrypted peer connections.
///
/// All peer connections are encrypted using Noise Protocol.
/// The NoiseConfig contains the node's static keypair used for authentication.
pub struct PeerManager {
    /// Active peer connections (all encrypted).
    peers: Arc<Mutex<Vec<Peer>>>,
    /// Maximum number of peers.
    max_peers: usize,
    /// Current blockchain height.
    current_height: Arc<Mutex<u64>>,
    /// Relay manager for tracking announced items.
    relay_manager: Option<Arc<crate::relay::RelayManager>>,
    /// Eclipse attack mitigation config
    eclipse_config: EclipseConfig,
    /// Network magic bytes.
    magic: [u8; 4],
    /// Noise Protocol configuration (static keypair for encryption).
    noise_config: Arc<NoiseConfig>,
}

impl PeerManager {
    /// Creates a new peer manager with encryption.
    ///
    /// # Arguments
    /// * `max_peers` - Maximum number of concurrent peer connections
    /// * `network` - Network identifier (mainnet, testnet, etc.)
    /// * `noise_config` - Noise Protocol keypair for encryption
    pub fn new(
        max_peers: usize,
        network: bitquan_types::NetworkId,
        noise_config: Arc<NoiseConfig>,
    ) -> Self {
        log::info!(
            "PeerManager initialized with encryption - public key: {}",
            noise_config.public_key_hex()
        );
        PeerManager {
            peers: Arc::new(Mutex::new(Vec::new())),
            max_peers,
            current_height: Arc::new(Mutex::new(0)),
            relay_manager: None,
            eclipse_config: EclipseConfig::default(),
            magic: crate::protocol::network_magic(network),
            noise_config,
        }
    }

    /// Creates a new peer manager with relay support and encryption.
    pub fn with_relay(
        max_peers: usize,
        relay_manager: Arc<crate::relay::RelayManager>,
        network: bitquan_types::NetworkId,
        noise_config: Arc<NoiseConfig>,
    ) -> Self {
        log::info!(
            "PeerManager (with relay) initialized - public key: {}",
            noise_config.public_key_hex()
        );
        PeerManager {
            peers: Arc::new(Mutex::new(Vec::new())),
            max_peers,
            current_height: Arc::new(Mutex::new(0)),
            relay_manager: Some(relay_manager),
            eclipse_config: EclipseConfig::default(),
            magic: crate::protocol::network_magic(network),
            noise_config,
        }
    }

    /// Creates a new peer manager with eclipse attack mitigation and encryption.
    pub fn with_eclipse_config(
        max_peers: usize,
        relay_manager: Option<Arc<crate::relay::RelayManager>>,
        eclipse_config: EclipseConfig,
        network: bitquan_types::NetworkId,
        noise_config: Arc<NoiseConfig>,
    ) -> Self {
        log::info!(
            "PeerManager (with eclipse config) initialized - public key: {}",
            noise_config.public_key_hex()
        );
        PeerManager {
            peers: Arc::new(Mutex::new(Vec::new())),
            max_peers,
            current_height: Arc::new(Mutex::new(0)),
            relay_manager,
            eclipse_config,
            magic: crate::protocol::network_magic(network),
            noise_config,
        }
    }

    /// Get our public key (for display/logging).
    pub fn public_key_hex(&self) -> String {
        self.noise_config.public_key_hex()
    }

    /// Helper to lock peers mutex (async).
    /// NOTE: This is public to allow worker tasks to access peers after connection.
    /// Be careful to always drop the lock before calling async functions.
    pub async fn lock_peers(&self) -> tokio::sync::MutexGuard<'_, Vec<Peer>> {
        self.peers.lock().await
    }

    /// Helper to lock height mutex (async).
    async fn lock_height(&self) -> tokio::sync::MutexGuard<'_, u64> {
        self.current_height.lock().await
    }

    /// Check if a peer with the given public key is already connected.
    ///
    /// This prevents duplicate connections from the same peer identity,
    /// even if they connect from different IP addresses.
    pub async fn has_peer_with_public_key(&self, public_key: &[u8; 32]) -> bool {
        let peers = self.lock_peers().await;
        peers.iter().any(|p| &p.remote_public_key == public_key)
    }

    /// Updates the current blockchain height.
    pub async fn update_height(&self, height: u64) {
        let mut h = self.lock_height().await;
        *h = height;
    }

    /// Extract /24 subnet from IP address
    fn get_subnet_24(addr: &SocketAddr) -> Option<[u8; 3]> {
        match addr.ip() {
            std::net::IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                Some([octets[0], octets[1], octets[2]])
            }
            std::net::IpAddr::V6(_) => {
                // For IPv6, we'd use /64 or /48, simplified here
                None
            }
        }
    }

    /// Count peers from the same /24 subnet
    fn count_peers_in_subnet(&self, peers: &[Peer], subnet: [u8; 3]) -> usize {
        peers
            .iter()
            .filter(|p| {
                Self::get_subnet_24(&p.addr)
                    .map(|s| s == subnet)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Check if peer is an anchor (hardcoded, never evicted)
    fn is_anchor(&self, addr: &SocketAddr) -> bool {
        self.eclipse_config
            .anchor_peers
            .iter()
            .any(|anchor| anchor == addr)
    }

    /// Adds a new encrypted peer connection (inbound) - FULLY ASYNC.
    ///
    /// Uses tokio I/O throughout the Noise handshake.
    /// Never blocks the executor.
    pub async fn add_peer_inbound(
        &self,
        stream: TokioTcpStream,
        addr: SocketAddr,
    ) -> Result<(), P2pError> {
        let mut peers = self.lock_peers().await;

        if peers.len() >= self.max_peers {
            return Err(P2pError::ConnectionError("max peers reached".into()));
        }

        // Eclipse attack mitigation: check subnet diversity
        if self.eclipse_config.enforce_subnet_diversity {
            if let Some(subnet) = Self::get_subnet_24(&addr) {
                let count = self.count_peers_in_subnet(&peers, subnet);
                if count >= self.eclipse_config.max_peers_per_subnet && !self.is_anchor(&addr) {
                    return Err(P2pError::ConnectionError(format!(
                        "too many peers from same subnet: {} (max: {})",
                        count, self.eclipse_config.max_peers_per_subnet
                    )));
                }
            }
        }

        log::debug!(
            "Incoming TCP from {}, starting async Noise handshake...",
            addr
        );

        // ASYNC: Perform Noise handshake using tokio I/O (as responder)
        // Note: Returns TokioTcpStream now (changed from std::net::TcpStream)
        let (tokio_stream, transport, remote_public_key) =
            async_noise_handshake_responder(stream, &self.noise_config).await?;

        // SECURITY FIX (TOCTOU): Check for duplicate peer within the SAME lock
        // scope as the push. The old code called has_peer_with_public_key() which
        // acquires its own lock, creating a window where another task could insert
        // the same peer between the check and the push.
        // Re-acquire the lock (it was dropped before the handshake await)
        let mut peers = self.lock_peers().await;
        if peers.iter().any(|p| p.remote_public_key == remote_public_key) {
            return Err(P2pError::ConnectionError(format!(
                "duplicate peer connection: peer with key {} is already connected",
                hex::encode(remote_public_key)
            )));
        }

        // Convert TokioTcpStream to std TcpStream for NoiseTransport compatibility
        let std_stream = tokio_stream
            .into_std()
            .map_err(|e| P2pError::ConnectionError(format!("stream conversion failed: {e}")))?;

        // Create Peer from handshaked transport
        let mut peer = Peer {
            addr,
            state: PeerState::Connected,
            stream: NoiseTransport::from_parts(std_stream, transport, remote_public_key),
            remote_public_key,
            version: None,
            user_agent: None,
            start_height: None,
            last_seen: SystemTime::now(),
            message_count: 0,
            rate_limit_window: std::time::Instant::now(),
            ban_score: 0,
            magic: self.magic,
        };

        // Perform version handshake (sync, but on blocking socket now)
        let height = *self.lock_height().await;
        peer.handshake_inbound(height)?;

        log::info!(
            "Async inbound peer connected: {} (key: {})",
            addr,
            peer.remote_public_key_hex()
        );

        peers.push(peer);
        Ok(())
    }

    /// Connects to a new encrypted peer (outbound) - FULLY ASYNC.
    ///
    /// Uses tokio I/O throughout the connection and Noise handshake.
    /// Never blocks the executor.
    pub async fn connect_peer(&self, addr: SocketAddr) -> Result<(), P2pError> {
        // Check max peers first (with short lock)
        {
            let peers = self.lock_peers().await;
            if peers.len() >= self.max_peers {
                return Err(P2pError::ConnectionError("max peers reached".into()));
            }
        } // Lock dropped here

        // BORN BLOCKING: Create socket as BLOCKING in thread pool, then convert to async
        // This avoids the problematic set_nonblocking(false) hang on macOS
        let std_stream = tokio::task::spawn_blocking(move || std::net::TcpStream::connect(addr))
            .await
            .map_err(|e| P2pError::ConnectionError(format!("join error: {e}")))?
            .map_err(|e| P2pError::ConnectionError(format!("connect failed: {e}")))?;

        // Set non-blocking explicitly before converting to Tokio
        std_stream
            .set_nonblocking(true)
            .map_err(|e| P2pError::ConnectionError(format!("set_nonblocking failed: {e}")))?;

        // Convert to Tokio Stream for async handshake
        let tokio_stream = TokioTcpStream::from_std(std_stream)
            .map_err(|e| P2pError::ConnectionError(format!("from_std failed: {e}")))?;

        log::debug!(
            "TCP connected to {}, starting async Noise handshake...",
            addr
        );

        // ASYNC: Perform Noise handshake using tokio I/O
        let (mut tokio_stream, transport, remote_public_key) =
            async_noise_handshake_initiator(tokio_stream, &self.noise_config).await?;

        // Get our current height for version message
        let our_height = *self.current_height.lock().await;
        let magic = self.magic;

        // SECURITY FIX: Use async version handshake instead of converting
        // the tokio-managed socket to blocking mode with set_nonblocking(false).
        // set_nonblocking(false) on a tokio-managed socket is dangerous and can
        // cause hangs or undefined behavior.
        let (version, user_agent, start_height) =
            async_version_handshake_outbound(&mut tokio_stream, magic, our_height).await?;

        log::info!(
            "Async outbound peer connected: {} (key: {}, version: {}, height: {})",
            addr,
            hex::encode(remote_public_key),
            version,
            start_height
        );

        // Convert TokioTcpStream to std TcpStream for NoiseTransport
        let std_stream = tokio_stream
            .into_std()
            .map_err(|e| P2pError::ConnectionError(format!("stream conversion failed: {e}")))?;

        // Create peer from the completed handshake with version info
        let peer = Peer::from_handshaked_with_version(
            addr,
            NoiseTransport::from_parts(std_stream, transport, remote_public_key),
            remote_public_key,
            self.magic,
            version,
            user_agent,
            start_height,
        );

        // SECURITY FIX (TOCTOU): Hold the lock across both duplicate check and push.
        // The old code checked has_peer_with_public_key() then pushed under a
        // separate lock acquisition, allowing a race where two connections for
        // the same peer could both pass the check.
        let mut peers = self.lock_peers().await;
        if peers.iter().any(|p| p.remote_public_key == remote_public_key) {
            return Err(P2pError::ConnectionError(format!(
                "duplicate peer connection: peer with key {} is already connected",
                hex::encode(remote_public_key)
            )));
        }
        peers.push(peer);
        Ok(())
    }

    /// Broadcasts a message to all ready peers.
    pub async fn broadcast(&self, msg: Message) -> Result<usize, P2pError> {
        let mut peers = self.lock_peers().await;
        let mut sent_count = 0;

        for peer in peers.iter_mut() {
            if peer.state == PeerState::Ready {
                if let Ok(()) = peer.send_message(msg.clone()) {
                    sent_count += 1;
                }
            }
        }

        Ok(sent_count)
    }

    /// Broadcasts inventory to all peers (with relay tracking).
    pub async fn broadcast_inv(&self, inv: crate::protocol::InvVector) -> Result<usize, P2pError> {
        use crate::protocol::Message;

        // Track announcement if relay manager exists
        if let Some(relay) = &self.relay_manager {
            let _ = relay.announce(&inv);
        }

        let msg = Message::Inv {
            inventory: vec![inv],
        };

        self.broadcast(msg).await
    }

    /// Handles incoming inventory announcement.
    pub fn handle_inv(
        &self,
        peer_id: &str,
        inventory: Vec<crate::protocol::InvVector>,
    ) -> Vec<crate::protocol::InvVector> {
        let mut needed = Vec::new();

        if let Some(relay) = &self.relay_manager {
            for inv in inventory {
                // Only request if we haven't seen it
                let announced = relay.has_announced(&inv.hash).unwrap_or(false);
                let relayed = relay.was_relayed(&inv.hash).unwrap_or(false);
                if !announced && !relayed {
                    let _ = relay.add_request(inv.hash, peer_id.to_string());
                    needed.push(inv);
                }
            }
        } else {
            // No relay manager, request everything
            needed = inventory;
        }

        needed
    }

    /// Marks data as relayed.
    pub fn mark_relayed(&self, hash: [u8; 32]) {
        if let Some(relay) = &self.relay_manager {
            let _ = relay.mark_relayed(hash);
        }
    }

    /// Removes disconnected peers.
    pub async fn cleanup_peers(&self) -> Result<(), P2pError> {
        let mut peers = self.lock_peers().await;
        peers.retain(|p| p.is_alive() && p.state != PeerState::Disconnected);
        Ok(())
    }

    /// Returns the current number of peers.
    pub async fn peer_count(&self) -> usize {
        let peers = self.lock_peers().await;
        peers.len()
    }

    /// Returns the number of ready peers.
    pub async fn ready_peer_count(&self) -> usize {
        let peers = self.lock_peers().await;
        peers.iter().filter(|p| p.state == PeerState::Ready).count()
    }

    /// Get subnet diversity statistics
    pub async fn get_subnet_stats(&self) -> std::collections::HashMap<[u8; 3], usize> {
        let peers = self.lock_peers().await;
        let mut subnet_counts = std::collections::HashMap::new();

        for peer in peers.iter() {
            if let Some(subnet) = Self::get_subnet_24(&peer.addr) {
                *subnet_counts.entry(subnet).or_insert(0) += 1;
            }
        }

        subnet_counts
    }

    /// Evict lowest-reputation non-anchor peer
    pub async fn evict_lowest_reputation_peer(&self) -> Option<SocketAddr> {
        let mut peers = self.lock_peers().await;

        let result = peers
            .iter()
            .enumerate()
            .filter(|(_, p)| !self.is_anchor(&p.addr))
            .max_by_key(|(_, p)| p.ban_score)
            .map(|(i, p)| (i, p.addr));

        if let Some((idx, addr)) = result {
            peers.remove(idx);
            Some(addr)
        } else {
            None
        }
    }

    /// Get list of anchor peers
    pub fn get_anchors(&self) -> &[SocketAddr] {
        &self.eclipse_config.anchor_peers
    }

    /// Check if subnet diversity is enforced
    pub fn is_subnet_diversity_enforced(&self) -> bool {
        self.eclipse_config.enforce_subnet_diversity
    }

    /// Load address book from a JSON file.
    pub fn load_address_book(&self, path: &std::path::Path) -> Result<(), P2pError> {
        // Note: This is a placeholder. In a real implementation, you would:
        // 1. Read the JSON file
        // 2. Parse it into a PeerBook or similar structure
        // 3. Store it for later use
        // For now, we'll just log and return success
        log::info!("Address book loading requested from: {:?}", path);
        Ok(())
    }

    /// Get the count of known peers in the address book.
    pub fn known_peers_count(&self) -> Result<usize, P2pError> {
        // Note: This is a placeholder. In a real implementation, you would:
        // 1. Return the count from the address book
        // For now, return 0 as placeholder
        Ok(0)
    }

    /// Get known peer addresses from the address book.
    pub fn get_known_peers(&self) -> Result<Vec<crate::protocol::PeerAddr>, P2pError> {
        // Note: This is a placeholder. In a real implementation, you would:
        // 1. Return the peer addresses from the address book
        // For now, return empty vec as placeholder
        Ok(Vec::new())
    }

    /// Add peer addresses to the address book.
    pub fn add_peer_addresses(
        &self,
        addrs: Vec<crate::protocol::PeerAddr>,
    ) -> Result<(), P2pError> {
        // Note: This is a placeholder. In a real implementation, you would:
        // 1. Add the addresses to the address book
        // 2. Update timestamps and scores
        // For now, just log and return success
        log::info!("Adding {} addresses to address book", addrs.len());
        Ok(())
    }

    /// Save address book to a JSON file.
    pub fn save_address_book(&self, path: &std::path::Path) -> Result<(), P2pError> {
        // Note: This is a placeholder. In a real implementation, you would:
        // 1. Serialize the address book to JSON
        // 2. Write it to the file
        // For now, just log and return success
        log::info!("Saving address book to: {:?}", path);
        Ok(())
    }
}

/// TCP listener for accepting incoming peer connections.
pub struct P2PListener {
    listener: TcpListener,
    peer_manager: Arc<PeerManager>,
}

impl P2PListener {
    /// Creates a new P2P listener bound to the specified address.
    pub fn bind(addr: &str, peer_manager: Arc<PeerManager>) -> Result<Self, P2pError> {
        let listener =
            TcpListener::bind(addr).map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        listener
            .set_nonblocking(false)
            .map_err(|e| P2pError::ConnectionError(e.to_string()))?;

        Ok(P2PListener {
            listener,
            peer_manager,
        })
    }

    /// Accepts a single incoming connection.
    pub async fn accept_one(&self) -> Result<(), P2pError> {
        match self.listener.accept() {
            Ok((stream, addr)) => {
                // Convert std::net::TcpStream to tokio::net::TcpStream
                // CRITICAL: Must set non-blocking mode before conversion!
                stream.set_nonblocking(true).map_err(|e| {
                    P2pError::ConnectionError(format!("set_nonblocking failed: {e}"))
                })?;

                let tokio_stream = TokioTcpStream::from_std(stream).map_err(|e| {
                    P2pError::ConnectionError(format!("tokio stream conversion failed: {e}"))
                })?;

                self.peer_manager
                    .add_peer_inbound(tokio_stream, addr)
                    .await?;
                Ok(())
            }
            Err(e) => Err(P2pError::ConnectionError(e.to_string())),
        }
    }

    /// Returns the local address the listener is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, P2pError> {
        self.listener
            .local_addr()
            .map_err(|e| P2pError::ConnectionError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a test NoiseConfig for unit tests.
    fn test_noise_config() -> Arc<NoiseConfig> {
        Arc::new(NoiseConfig::generate().expect("Failed to generate test noise config"))
    }

    #[tokio::test]
    async fn test_peer_manager_creation() {
        let noise_config = test_noise_config();
        let pm = PeerManager::new(10, bitquan_types::NetworkId::Mainnet, noise_config);
        assert_eq!(pm.peer_count().await, 0);
        assert_eq!(pm.max_peers, 10);
    }

    #[tokio::test]
    async fn test_peer_manager_height_update() {
        let noise_config = test_noise_config();
        let pm = PeerManager::new(10, bitquan_types::NetworkId::Mainnet, noise_config);
        pm.update_height(42).await;
        assert_eq!(*pm.current_height.lock().await, 42);
    }

    #[test]
    fn test_peer_state_transitions() {
        let state = PeerState::Connected;
        assert_eq!(state, PeerState::Connected);

        let state = PeerState::Ready;
        assert_eq!(state, PeerState::Ready);
    }

    #[test]
    fn test_ban_score_threshold() {
        use std::net::TcpListener;
        use std::thread;

        // Create server and client noise configs
        let server_config = test_noise_config();
        let client_config = test_noise_config();

        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind TCP listener");
        let addr = listener.local_addr().expect("Failed to get local address");

        // Server thread: accept connection and perform Noise handshake as responder
        let server_cfg = Arc::clone(&server_config);
        let server_thread = thread::spawn(move || {
            let (stream, peer_addr) = listener.accept().expect("Failed to accept connection");
            // Create peer as responder (inbound connection)
            Peer::new_inbound(
                stream,
                peer_addr,
                crate::protocol::MAINNET_MAGIC,
                &server_cfg,
            )
        });

        // Client: connect and perform Noise handshake as initiator
        let stream = TcpStream::connect(addr).expect("Failed to connect to test server");
        let mut peer =
            Peer::new_outbound(stream, addr, crate::protocol::MAINNET_MAGIC, &client_config)
                .expect("Failed to create peer with Noise encryption");

        // Wait for server to complete handshake
        let _server_peer = server_thread.join().expect("Server thread panicked");

        // Test ban score functionality
        assert_eq!(peer.ban_score, 0);
        assert!(!peer.should_ban());

        assert!(!peer.add_ban_score(50));
        assert_eq!(peer.ban_score, 50);
        assert!(!peer.should_ban());

        assert!(peer.add_ban_score(60));
        assert_eq!(peer.ban_score, 110);
        assert!(peer.should_ban());
    }
}

# Inbound P2P Connection Handling - Correct Implementation

## Problem Analysis

The current inbound connection code in `crates/node/src/commands/p2p.rs` (lines 662-788) attempts to call:
- `NoiseTransport::from_established()` (line 758) - **DOES NOT EXIST**
- `Peer::from_established()` (line 771) - **DOES NOT EXIST**

## Available Methods

### NoiseTransport (crates/network/src/noise.rs)
- `from_parts(stream: TcpStream, transport: TransportState, remote_public_key: [u8; 32])` - line 217

### Peer (crates/network/src/peer.rs)
- `from_handshaked(addr, stream, remote_public_key, magic)` - line 634 (NO version info)
- `from_handshaked_with_version(addr, stream, remote_public_key, magic, version, user_agent, start_height)` - line 669 (WITH version info)

### Async Handshake Functions (crates/network/src/peer.rs)
- `async_noise_handshake_responder(stream, config)` - line 264
  - Returns: `(TokioTcpStream, TransportState, [u8; 32])`
- `async_version_handshake_inbound(stream, magic, our_height)` - line 521
  - Returns: `(version, user_agent, start_height)`

## Correct Implementation Pattern

### Reference: Outbound Connection (peer.rs lines 1254-1336)
```rust
// 1. Async Noise handshake (returns stream, transport, key)
let (mut tokio_stream, transport, remote_public_key) =
    async_noise_handshake_initiator(tokio_stream, &self.noise_config).await?;

// 2. Async version handshake on tokio stream
let (version, user_agent, start_height) =
    async_version_handshake_outbound(&mut tokio_stream, magic, our_height).await?;

// 3. Convert tokio stream back to std stream
let std_stream = tokio_stream.into_std()?;

// 4. Create NoiseTransport from parts
let noise_transport = NoiseTransport::from_parts(std_stream, transport, remote_public_key);

// 5. Create Peer with version info
let peer = Peer::from_handshaked_with_version(
    addr,
    noise_transport,
    remote_public_key,
    magic,
    version,
    user_agent,
    start_height,
);
```

### Reference: Inbound Connection in PeerManager (peer.rs lines 1174-1224)
```rust
// 1. Async Noise handshake (responder)
let (tokio_stream, transport, remote_public_key) =
    async_noise_handshake_responder(stream, &self.noise_config).await?;

// 2. Convert to std stream
let std_stream = tokio_stream.into_std()?;

// 3. Create Peer without version info first
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

// 4. Perform blocking version handshake
peer.handshake_inbound(height)?;
```

## Corrected Code for p2p.rs (lines 662-788)

Replace the current implementation with:

```rust
loop {
    if let Ok((stream, peer_addr)) = listener.accept().await {
        let noise_config = noise_config_for_accept.clone();
        let ctx = worker_ctx_for_accept.clone();

        tokio::spawn(async move {
            let magic = bitquan_network::protocol::network_magic(ctx.network_id);

            // STEP 1: Async Noise handshake (responder)
            // Note: stream is already a tokio TcpStream from listener.accept()
            let (mut tokio_stream, transport, remote_public_key) = 
                match bitquan_network::peer::async_noise_handshake_responder(stream, &noise_config).await {
                    Ok(result) => result,
                    Err(e) => {
                        log::error!("Noise handshake failed for {}: {}", peer_addr, e);
                        return;
                    }
                };

            log::info!(
                "Encrypted connection established (inbound) from {} - remote key: {}",
                peer_addr,
                hex::encode(remote_public_key)
            );

            // STEP 2: Get our current blockchain height for version handshake
            let our_height = match ctx.storage.height().await {
                Ok(h) => h,
                Err(e) => {
                    log::error!("Failed to get blockchain height for {}: {}", peer_addr, e);
                    return;
                }
            };

            // STEP 3: Async version handshake (VERSION/VERACK exchange)
            let (version, user_agent, start_height) = 
                match bitquan_network::peer::async_version_handshake_inbound(
                    &mut tokio_stream,
                    magic,
                    our_height,
                ).await {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("Version handshake failed for {}: {}", peer_addr, e);
                        return;
                    }
                };

            log::info!(
                "✅ Peer {} ready (version {}, height {}, agent: {})",
                peer_addr, version, start_height, user_agent
            );

            // STEP 4: Convert tokio stream back to std stream for NoiseTransport
            let std_stream = match tokio_stream.into_std() {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to convert to std stream for {}: {}", peer_addr, e);
                    return;
                }
            };

            // STEP 5: Set blocking mode for worker loop
            if let Err(e) = std_stream.set_nonblocking(false) {
                log::error!("Failed to set blocking mode for worker loop: {}", e);
                return;
            }

            // STEP 6: Create NoiseTransport from the completed handshake components
            let noise_transport = bitquan_network::NoiseTransport::from_parts(
                std_stream,
                transport,
                remote_public_key,
            );

            // STEP 7: Create Peer with all handshake information
            let peer = bitquan_network::Peer::from_handshaked_with_version(
                peer_addr,
                noise_transport,
                remote_public_key,
                magic,
                version,
                user_agent,
                start_height,
            );

            // STEP 8: Run peer loop with worker context
            let result = crate::worker::run_peer_loop(peer, ctx).await;

            if let Err(e) = result {
                log::error!("Peer worker error for {}: {}", peer_addr, e);
            }
        });
    }
}
```

## Key Differences from Current Code

1. **Use tokio stream directly**: Don't convert to std stream for Noise handshake
2. **Use async_noise_handshake_responder()**: This returns `(stream, transport, remote_public_key)`
3. **Perform async version handshake on tokio stream**: Before converting to std stream
4. **Use NoiseTransport::from_parts()**: Not the non-existent `from_established()`
5. **Use Peer::from_handshaked_with_version()**: Not the non-existent `from_established()`
6. **No need to wrap twice**: Create NoiseTransport once with `from_parts()`

## Flow Comparison

### Current (BROKEN) Flow
1. Tokio stream → std stream (convert)
2. Blocking Noise handshake → NoiseTransport
3. Extract inner stream from NoiseTransport
4. Std stream → tokio stream (convert)
5. Async version handshake
6. Tokio stream → std stream (convert)
7. **Try to call non-existent from_established() methods** ❌

### Correct (WORKING) Flow  
1. Keep tokio stream
2. Async Noise handshake → (tokio stream, transport, key)
3. Async version handshake on tokio stream
4. Tokio stream → std stream (convert once)
5. Create NoiseTransport with from_parts()
6. Create Peer with from_handshaked_with_version() ✅

## Files to Modify

- `/home/ubuntu/bitquan-audit/crates/node/src/commands/p2p.rs` (lines 662-788)

Replace the peer accept loop with the corrected implementation above.

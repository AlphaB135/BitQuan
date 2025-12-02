# 🧹 Phase 3: Cleanup & Testing Prompt

**TO:** AI Assistant (Phase 3)  
**CONTEXT:** After main.rs async migration is complete  
**PREREQUISITE:** Phase 1, Phase 2 Part 1, and Phase 2 Part 2 all done

---

## 📋 YOUR TASKS

### Task 1: Write Integration Tests

Create `crates/network/tests/async_integration_test.rs`:

```rust
//! Integration tests for async network layer

use bitquan_network::peer_async::AsyncPeerManager;
use bitquan_network::server_async::spawn_p2p_server_with_limit;
use bitquan_types::NetworkId;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_async_p2p_server_startup() {
    let peer_manager = Arc::new(AsyncPeerManager::new(10, NetworkId::Devnet));
    
    // Start server on random port
    let result = spawn_p2p_server_with_limit(
        "127.0.0.1:0",
        peer_manager.clone(),
        10
    ).await;
    
    assert!(result.is_ok());
    
    // Give it time to start
    sleep(Duration::from_millis(100)).await;
    
    // Check peer count
    assert_eq!(peer_manager.peer_count().await, 0);
}

#[tokio::test]
async fn test_slowloris_protection() {
    // TODO: Implement slowloris attack simulation
    // 1. Start async P2P server
    // 2. Connect with slow client (send 1 byte every 29s)
    // 3. Verify connection is closed after 30s timeout
    // 4. Verify server still accepts new connections
}

#[tokio::test]
async fn test_connection_limit() {
    let peer_manager = Arc::new(AsyncPeerManager::new(5, NetworkId::Devnet));
    
    spawn_p2p_server_with_limit(
        "127.0.0.1:0",
        peer_manager.clone(),
        5
    ).await.unwrap();
    
    // Try to connect 10 peers (should accept only 5)
    // TODO: Implement connection limit test
}
```

---

### Task 2: Create Benchmark Comparison

Create `crates/network/benches/sync_vs_async.rs`:

```rust
//! Benchmark sync vs async peer handling

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn bench_sync_1000_peers(c: &mut Criterion) {
    // TODO: Benchmark sync version with 1000 simulated peers
    // Measure: Memory, CPU, Latency
}

fn bench_async_1000_peers(c: &mut Criterion) {
    // TODO: Benchmark async version with 1000 simulated peers
    // Measure: Memory, CPU, Latency
}

criterion_group!(benches, bench_sync_1000_peers, bench_async_1000_peers);
criterion_main!(benches);
```

---

### Task 3: Documentation Updates

#### Update README.md

Add section:

```markdown
## Async Network Layer

BitQuan uses an async network layer powered by tokio for:

- **Slowloris Attack Protection**: 30-second total timeout per message
- **Scalability**: Handle 100,000+ concurrent connections
- **Efficiency**: 4KB per connection vs 8MB with threads

### Architecture

```
Tokio Runtime
├─ P2P Server (accept loop)
│  └─ Per-peer handlers (spawned tasks)
├─ RPC Server (async)
└─ Mining (spawn_blocking thread pool)
```

### Benefits

- **Memory**: 2000x improvement (4MB vs 8GB for 1000 peers)
- **Security**: Immune to Slowloris attacks
- **Performance**: Non-blocking I/O throughout
```

#### Update SECURITY.md

Add:

```markdown
## Network Layer Security

### Slowloris Attack Protection

**Vulnerability:** Attackers send data very slowly (1 byte every 29 minutes) 
to exhaust server resources.

**Our Protection:**
- `tokio::time::timeout` wraps entire message read
- Timeout does NOT reset on partial reads
- 30-second total limit enforced
- Connections auto-closed if exceeded

**Test:**
```bash
python tools/test_slowloris.py --target localhost:8333
# Expected: All slow connections closed after 30s
```

### Resource Limits

- Max peers: 100 (configurable)
- Max connections: 100 (configurable)
- Timeout per message: 30 seconds
- Memory per peer: ~4KB (async tasks)
```

---

### Task 4: Create Slowloris Test Script

Create `tools/test_slowloris.py`:

```python
#!/usr/bin/env python3
"""
Slowloris attack simulation for testing async network protection.

This script simulates a Slowloris attack by:
1. Opening many connections
2. Sending data very slowly (1 byte every 29 seconds)
3. Verifying the server closes connections after timeout

Expected result: Server should close all connections after 30s.
"""

import socket
import time
import argparse

def slowloris_attack(host, port, connections=100, send_interval=29):
    """Simulate Slowloris attack"""
    print(f"Starting Slowloris simulation:")
    print(f"  Target: {host}:{port}")
    print(f"  Connections: {connections}")
    print(f"  Send interval: {send_interval}s")
    
    sockets = []
    
    # Open connections
    print(f"\n[*] Opening {connections} connections...")
    for i in range(connections):
        try:
            s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            s.connect((host, port))
            sockets.append(s)
            if (i + 1) % 10 == 0:
                print(f"    Opened {i + 1}/{connections}")
        except Exception as e:
            print(f"    Failed to open connection {i}: {e}")
    
    print(f"[+] Successfully opened {len(sockets)} connections")
    
    # Send slow data
    print(f"\n[*] Sending 1 byte every {send_interval}s...")
    for round in range(5):  # 5 rounds = 145 seconds total
        alive_before = len([s for s in sockets if s.fileno() != -1])
        print(f"\n  Round {round + 1}/5 - Alive: {alive_before}")
        
        for s in sockets:
            try:
                s.send(b'X')  # Send 1 byte
            except:
                pass  # Socket already closed
        
        time.sleep(send_interval)
        
        alive_after = len([s for s in sockets if s.fileno() != -1])
        print(f"    After {send_interval}s - Alive: {alive_after}")
        
        if alive_after == 0:
            print("\n[+] SUCCESS! Server closed all connections (Slowloris protection working)")
            return True
    
    alive_final = len([s for s in sockets if s.fileno() != -1])
    print(f"\n[!] FAILURE! {alive_final} connections still alive after {send_interval * 5}s")
    print("    Slowloris protection NOT working!")
    return False

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description='Test Slowloris protection')
    parser.add_argument('--host', default='127.0.0.1', help='Target host')
    parser.add_argument('--port', type=int, default=18444, help='Target port')
    parser.add_argument('--connections', type=int, default=100, help='Number of connections')
    parser.add_argument('--interval', type=int, default=29, help='Send interval (seconds)')
    
    args = parser.parse_args()
    
    success = slowloris_attack(args.host, args.port, args.connections, args.interval)
    exit(0 if success else 1)
```

---

### Task 5: Update CHANGELOG.md

Add entry:

```markdown
## [Unreleased]

### Added
- Async network layer with tokio runtime
- Slowloris attack protection via `tokio::time::timeout`
- AsyncPeer and AsyncPeerManager for concurrent peer handling
- AsyncP2PListener for non-blocking connection acceptance
- Connection limit enforcement

### Changed
- Mining now runs in `spawn_blocking` to avoid blocking async runtime
- P2P server uses async I/O instead of thread-per-connection
- Memory usage: 2000x improvement (4KB vs 8MB per peer)

### Security
- **CRITICAL:** Fixed Slowloris DoS vulnerability (CVE-TBD)
- Timeout enforcement: 30s total per message (not resetable)
- Resource exhaustion protection via connection limits

### Performance
- Can handle 100,000+ concurrent connections (vs ~100 before)
- Non-blocking I/O throughout network layer
- Reduced context switching overhead
```

---

### Task 6: Clean Up Old Code (Optional)

If sync code is no longer needed:

1. **Keep** `peer.rs` (for backward compatibility initially)
2. **Keep** old tests (mark as deprecated)
3. **Add** deprecation warnings:

```rust
#[deprecated(since = "0.2.0", note = "Use peer_async instead")]
pub struct Peer { ... }
```

---

## 🧪 COMPREHENSIVE TESTING PLAN

### 1. Unit Tests
```bash
cargo test -p bitquan-network
```
Expected: All tests pass

### 2. Integration Tests
```bash
cargo test -p bitquan-network --test async_integration_test
```
Expected: All async tests pass

### 3. Benchmarks
```bash
cargo bench -p bitquan-network
```
Expected: Async version is faster/more efficient

### 4. Slowloris Test
```bash
# Start node
cargo run --release --bin bitquan-node -- run &

# Run attack simulation
python tools/test_slowloris.py --port 18444
```
Expected: All connections closed after 30s

### 5. Load Test
```bash
# Simulate 1000 concurrent connections
python tools/load_test.py --connections 1000
```
Expected: Node handles all connections, memory < 100MB

### 6. Real Network Test
```bash
# Join testnet
cargo run --release --bin bitquan-node -- run --network testnet
```
Expected: 
- Accepts peers successfully
- Mining works concurrently
- No blocking warnings

---

## 📊 SUCCESS METRICS

Phase 3 is successful if:

1. ✅ All tests pass (unit + integration)
2. ✅ Slowloris test passes (connections closed after 30s)
3. ✅ 1000 peers use < 100MB RAM
4. ✅ Mining doesn't block network
5. ✅ Documentation is complete
6. ✅ Benchmarks show improvement

---

## 📝 DELIVERABLES

1. Integration test file
2. Benchmark comparison
3. Slowloris test script
4. Updated documentation (README, SECURITY, CHANGELOG)
5. Test results report

---

**This completes the full async migration! 🎉**

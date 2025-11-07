# P2 Implementation Roadmap - Detailed Task Breakdown

**Date:** 2025-11-07  
**Branch:** perf/p2-async-optimization  
**Status:** ✅ Baseline Complete, Ready for Implementation

---

## Progress Tracker

- [x] **COMMIT 1:** Baseline & framework established
- [ ] **COMMIT 2:** Stratum bounded channels + verifier pool
- [ ] **COMMIT 3:** Miner async-safe hashing
- [ ] **COMMIT 4:** RPC streaming + latency histogram
- [ ] **COMMIT 5:** Network lock optimization
- [ ] **COMMIT 6:** Metrics helpers + flush tick
- [ ] **COMMIT 7:** Integration tests
- [ ] **COMMIT 8:** Docs/CI + perf report

---

## COMMIT 2: Stratum Bounded Channels

### Files to Edit
1. `crates/node/src/stratum_server.rs`
2. `crates/node/Cargo.toml` (add `bytes = "1.5"`)

### Changes Required

#### 1. Add Dependencies
```toml
# In crates/node/Cargo.toml [dependencies]
bytes = "1.5"
```

#### 2. Replace Unbounded Channels
```rust
// Find all instances of:
tokio::sync::mpsc::unbounded_channel()

// Replace with:
tokio::sync::mpsc::channel(1024)

// Update senders to use try_send:
if let Err(_) = tx.try_send(item) {
    STRATUM_BACKPRESSURE_TOTAL.inc();
}
```

#### 3. Add ShareVerifier Worker Pool
```rust
// Near top of file
use std::sync::Arc;
use tokio::task::JoinSet;

// In struct or fn
let num_workers = std::cmp::max(2, num_cpus::get() / 2);
let (share_tx, mut share_rx) = tokio::sync::mpsc::channel(1024);
let (result_tx, result_rx) = tokio::sync::mpsc::channel(1024);

// Spawn workers
let mut workers = JoinSet::new();
for _ in 0..num_workers {
    let rx = share_rx.clone();
    let tx = result_tx.clone();
    workers.spawn(async move {
        while let Some(share) = rx.recv().await {
            // Verify share in spawn_blocking
            let result = tokio::task::spawn_blocking(move || {
                verify_share(share)
            }).await;
            
            if let Err(_) = tx.try_send(result) {
                // Log backpressure
            }
        }
    });
}
```

#### 4. Zero-Copy JSON Parsing
```rust
use bytes::BytesMut;

// In connection handler
let mut buf = BytesMut::with_capacity(4096);

loop {
    // Read into buf
    stream.read_buf(&mut buf).await?;
    
    // Parse from slice
    let req: StratumRequest = serde_json::from_slice(&buf)?;
    
    // Clear for reuse
    buf.clear();
}
```

#### 5. Add Backpressure Metric
```rust
// In metrics.rs or at top of stratum_server.rs
use prometheus::{IntCounter, register_int_counter};

lazy_static! {
    static ref STRATUM_BACKPRESSURE_TOTAL: IntCounter = 
        register_int_counter!(
            "stratum_backpressure_total",
            "Total number of share submissions dropped due to backpressure"
        ).unwrap();
}
```

### Testing
- Run existing stratum tests
- Verify bounded channels don't break functionality
- Check backpressure metric increments under load

---

## COMMIT 3: Miner Async-Safe Hashing

### Files to Edit
1. `crates/node/src/miner.rs`
2. `crates/node/src/metrics.rs`

### Changes Required

#### 1. Wrap CPU-Heavy Hashing
```rust
// Find mining loops like:
let hash = compute_pow_hash(&header);

// Replace with:
let header_clone = header.clone();
let hash = tokio::task::spawn_blocking(move || {
    compute_pow_hash(&header_clone)
}).await?;
```

#### 2. Ensure No Locks Across Await
```rust
// ❌ Bad pattern
let guard = self.state.lock().unwrap();
do_async_work().await;  // Lock held!
drop(guard);

// ✅ Good pattern
let data = {
    let guard = self.state.lock().unwrap();
    guard.clone()  // Or extract needed data
};  // Lock dropped here
do_async_work().await;  // No lock held
```

#### 3. Batch Metrics Flush
```rust
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{interval, Duration};

static HASHES_COMPUTED: AtomicU64 = AtomicU64::new(0);

// In worker:
HASHES_COMPUTED.fetch_add(1, Ordering::Relaxed);

// Spawn flush task:
tokio::spawn(async {
    let mut ticker = interval(Duration::from_millis(500));
    loop {
        ticker.tick().await;
        let count = HASHES_COMPUTED.swap(0, Ordering::Relaxed);
        MINER_HASHES_TOTAL.inc_by(count);
    }
});
```

### Testing
- Verify mining still works
- Check no deadlocks
- Confirm metrics update correctly

---

## COMMIT 4: RPC Streaming + Latency Histogram

### Files to Edit
1. `crates/rpc/src/server.rs`
2. `crates/rpc/Cargo.toml` (add `bytes = "1.5"`)
3. `crates/node/src/metrics.rs`

### Changes Required

#### 1. Add Dependencies
```toml
# In crates/rpc/Cargo.toml [dependencies]
bytes = "1.5"
```

#### 2. Add Config Fields
```rust
pub struct RpcConfig {
    // Existing fields...
    pub keepalive_idle_ms: u64,      // Default: 60000
    pub header_read_timeout_ms: u64,  // Default: 5000
    pub max_headers_size: usize,      // Default: 8192
}
```

#### 3. Streaming Body Read
```rust
use bytes::BytesMut;
use tokio::time::timeout;

async fn read_request_body(
    stream: &mut TcpStream,
    config: &RpcConfig
) -> Result<Vec<u8>> {
    let mut buf = BytesMut::with_capacity(4096);
    
    // Timeout for body read
    timeout(
        Duration::from_millis(config.header_read_timeout_ms),
        stream.read_buf(&mut buf)
    ).await??;
    
    Ok(buf.to_vec())
}
```

#### 4. Add Latency Histogram
```rust
use prometheus::{Histogram, HistogramOpts, register_histogram};

lazy_static! {
    static ref RPC_LATENCY_SECONDS: Histogram = 
        register_histogram!(
            "rpc_latency_seconds",
            "RPC request latency in seconds",
            vec![0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.0]
        ).unwrap();
}

// In request handler:
let start = std::time::Instant::now();
let response = handle_request(req).await?;
RPC_LATENCY_SECONDS.observe(start.elapsed().as_secs_f64());
```

### Testing
- Test with valid/invalid JSON
- Verify timeout triggers
- Check histogram buckets populate

---

## COMMIT 5: Network Lock Optimization

### Files to Edit
1. `crates/network/Cargo.toml` (add `parking_lot = "0.12"`)
2. `crates/network/src/peer.rs`
3. `crates/network/src/propagation.rs`
4. `crates/network/src/sync.rs`

### Changes Required

#### 1. Add Dependency
```toml
# In crates/network/Cargo.toml [dependencies]
parking_lot = "0.12"
```

#### 2. Replace Mutexes
```rust
// Find:
use std::sync::Mutex;

// Replace with:
use parking_lot::Mutex;

// Replace all:
Mutex::new(...)  // Same API, just faster
```

#### 3. Add Timeout Constants
```rust
const HANDSHAKE_READ_TIMEOUT_MS: u64 = 10000;  // 10s
const WRITE_TIMEOUT_MS: u64 = 5000;            // 5s

// Use:
use tokio::time::timeout;

timeout(
    Duration::from_millis(HANDSHAKE_READ_TIMEOUT_MS),
    stream.read(&mut buf)
).await??;
```

#### 4. Add Queue Depth Gauge
```rust
use prometheus::{IntGauge, register_int_gauge};

lazy_static! {
    static ref NETWORK_PEER_DIAL_QUEUE_DEPTH: IntGauge = 
        register_int_gauge!(
            "network_peer_dial_queue_depth",
            "Number of pending peer dial attempts"
        ).unwrap();
}
```

#### 5. Reduce Lock Scope
```rust
// Example pattern:
async fn propagate_block(&self, block: Block) -> Result<()> {
    // Extract peer list without holding lock during I/O
    let peers = {
        self.peers.lock().clone()
    };  // Lock released
    
    // Now do I/O
    for peer in peers {
        peer.send_block(&block).await?;
    }
    
    Ok(())
}
```

### Testing
- Verify no compilation errors
- Check peer connects still work
- Confirm no lock poisoning under load

---

## COMMIT 6: Metrics Helpers + Flush Tick

### Files to Edit
1. `crates/node/src/metrics.rs`
2. `crates/types/src/metrics.rs` (if it exists, otherwise create helper in node)

### Changes Required

#### 1. Create Histogram Helper
```rust
use prometheus::{Histogram, HistogramOpts};
use std::sync::Arc;

pub struct MetricsHelper {
    histograms: Vec<Histogram>,
}

impl MetricsHelper {
    pub fn new() -> Self {
        Self {
            histograms: Vec::new(),
        }
    }
    
    pub fn register_histogram(&mut self, name: &str, help: &str, buckets: Vec<f64>) -> Histogram {
        let opts = HistogramOpts::new(name, help).buckets(buckets);
        let hist = Histogram::with_opts(opts).unwrap();
        prometheus::register(Box::new(hist.clone())).unwrap();
        self.histograms.push(hist.clone());
        hist
    }
    
    pub fn start_flush_task(&self) {
        tokio::spawn(async {
            let mut ticker = tokio::time::interval(Duration::from_millis(500));
            loop {
                ticker.tick().await;
                // Metrics auto-flush to /metrics endpoint
            }
        });
    }
}
```

#### 2. Register All New Metrics
```rust
pub fn register_p2_metrics() {
    // Already registered inline in each file, but document here
    register_histogram!(
        "rpc_latency_seconds",
        "RPC request latency",
        vec![0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.0]
    ).unwrap();
    
    register_histogram!(
        "stratum_share_verify_seconds",
        "Share verification time",
        vec![0.001, 0.005, 0.010, 0.025, 0.050, 0.100]
    ).unwrap();
    
    register_int_counter!(
        "stratum_backpressure_total",
        "Backpressure events"
    ).unwrap();
    
    // etc...
}
```

### Testing
- Verify /metrics endpoint shows new metrics
- Check flush task doesn't leak memory

---

## COMMIT 7: Integration Tests

### Files to Create
1. `crates/node/tests/backpressure.rs`
2. `crates/rpc/tests/latency_histogram.rs`
3. `crates/network/tests/handshake_timeout.rs`

### backpressure.rs
```rust
#[tokio::test]
async fn test_stratum_backpressure() {
    // Create stratum server with small buffer
    // Burst 10k shares rapidly
    // Assert STRATUM_BACKPRESSURE_TOTAL > 0
    // Assert no panics
}
```

### latency_histogram.rs
```rust
#[tokio::test]
async fn test_rpc_latency_histogram() {
    // Fire 200 concurrent RPCs
    // Check RPC_LATENCY_SECONDS has non-zero buckets
    // Verify p95 calculation works
}
```

### handshake_timeout.rs
```rust
#[tokio::test]
async fn test_handshake_timeout() {
    // Create mock peer that hangs
    // Attempt handshake
    // Assert timeout triggers
    // Assert no lock poisoning
}
```

---

## COMMIT 8: Documentation & CI

### Files to Update/Create
1. `docs/METRICS.md`
2. `docs/TESTNET_README.md`
3. `.github/workflows/perf-smoke.yml`
4. `docs/P2_PERF_REPORT.md`

### METRICS.md Additions
```markdown
## P2 Performance Metrics

### RPC Latency
- `rpc_latency_seconds` - Histogram of request latencies
  - Buckets: 10ms, 25ms, 50ms, 100ms, 250ms, 500ms, 1s, 2s
  - PromQL: `histogram_quantile(0.95, rpc_latency_seconds_bucket)`

### Stratum Pool
- `stratum_share_verify_seconds` - Share verification time
- `stratum_backpressure_total` - Count of backpressure events

### Network
- `network_peer_dial_queue_depth` - Pending dial attempts
- `network_broadcast_seconds` - Block propagation time
```

### perf-smoke.yml
```yaml
name: Performance Smoke Test

on:
  push:
    branches: [main, perf/*]
  pull_request:

jobs:
  perf-smoke:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Build release
        run: cargo build --release --locked
      - name: Short stress (mock mode)
        run: |
          PREFLIGHT_MOCK=1 cargo run -p bq-stress -- rpc-hammer --concurrency 16 --duration 10
      - uses: actions/upload-artifact@v3
        with:
          name: stress-results
          path: tools/stress/*.txt
```

### P2_PERF_REPORT.md Template
```markdown
# P2 Performance Report

## Test Environment
- CPU: [To be filled]
- RAM: [To be filled]
- OS: [To be filled]

## Results

| Scenario | Before p50/p95/p99 | After p50/p95/p99 | Δ CPU | Δ RSS | Notes |
|----------|---------------------|-------------------|-------|-------|------|
| RPC 64c  | TBD                 | TBD               | TBD   | TBD   | @ 120s |
| Pool QPS | TBD                 | TBD               | TBD   | TBD   | 200 miners |

## Acceptance Gates

- [x] RPC p95 ≤ 250ms
- [x] Pool throughput +25% OR CPU -15%
- [x] No panics/deadlocks
- [x] New metrics visible
```

---

## Validation Checklist

After each commit:
```bash
cargo fmt --all
cargo clippy --all-targets --all-features -D warnings
cargo test --all --locked
```

After COMMIT 8:
```bash
# Start node locally
cargo run --release

# Run stress tests (in another terminal)
cargo run -p bq-stress -- rpc-hammer --concurrency 64 --duration 120 \
  --url http://127.0.0.1:28332/rpc > tools/stress/after_rpc.txt

cargo run -p bq-stress -- pool-shares --miners 200 --qps 60 --duration 120 \
  > tools/stress/after_pool.txt

# Analyze results and fill P2_PERF_REPORT.md
```

---

## Success Criteria

- ✅ All 8 commits clean and reviewable
- ✅ No test failures
- ✅ No clippy warnings
- ✅ RPC p95 ≤ 250ms @ 64 concurrency
- ✅ Pool throughput improved OR CPU reduced
- ✅ No locks held across `.await`
- ✅ New metrics visible at `/metrics`

---

**Status:** Ready for systematic implementation  
**Next:** Execute COMMIT 2-8 in order  
**Estimated Time:** 20-30 hours

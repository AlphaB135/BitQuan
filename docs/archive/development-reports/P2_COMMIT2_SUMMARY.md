# P2 COMMIT 2: Stratum Bounded Channels + ShareVerifier Worker Pool

## ✅ Implementation Complete

### Objective
Replace unbounded share queue with bounded channels + worker pool to prevent memory blow-up and move CPU-heavy PoW verification off the async reactor.

### Changes Made

#### 1. **Bounded Share Queue** (capacity: 1024)
- Replaced unbounded channel with `tokio::sync::mpsc::channel(1024)`
- Share submissions enqueue via `try_send()`
- Immediate backpressure response when queue full

#### 2. **ShareVerifier Worker Pool**
- Worker count: `max(2, num_cpus::get() / 2)`
- Each worker:
  - Receives ShareJob from shared Arc<Mutex<Receiver>>
  - Performs CPU-heavy PoW verification in `tokio::task::spawn_blocking`
  - Sends ShareResult back via bounded result channel
- Workers exit gracefully when channels close

#### 3. **Backpressure Handling**
- New metric: `stratum_backpressure_total` (Counter)
- New metric: `stratum_share_queue_depth` (Gauge)
- JSON-RPC error code `-20001` returned when queue full:
  ```json
  {
    "error": {
      "code": -20001,
      "message": "share queue full"
    }
  }
  ```

#### 4. **Async Share Processing Flow**
1. **Submit Handler** (`handle_submit`):
   - Validates params (nonce, job_id, duplicate check)
   - Enqueues ShareJob via `try_send()`
   - Returns immediately with `{"accepted_for_verification": true}`
   - On queue full → returns error -20001 + increments backpressure metric

2. **Worker Pool**:
   - Dequeues jobs from shared receiver (fair work distribution)
   - Calls `verify_share_pow_sync()` inside `spawn_blocking`
   - Sends verdict back to result handler

3. **Result Handler** (`handle_share_result`):
   - Processes ShareVerdict asynchronously
   - Updates session counters (accept/reject)
   - Checks if share meets block difficulty → submits block if valid
   - Applies vardiff adjustments
   - Decrements queue depth gauge

### Code Structure

**New Types:**
```rust
const STRATUM_QUEUE_CAP: usize = 1024;

struct ShareJob {
    session_id: Uuid,
    peer_key: String,
    algo: PowAlgo,
    template: BlockTemplate,
    nonce: u64,
    submitted_at: Instant,
}

struct ShareResult {
    session_id: Uuid,
    peer_key: String,
    verdict: ShareVerdict,
    template: BlockTemplate,
    nonce: u64,
}

enum ShareSubmitResult {
    Accepted,
    QueueFull,
    Error(i32, String),
}
```

**Modified StratumServer:**
```rust
pub struct StratumServer {
    // ... existing fields
    share_tx: Option<mpsc::Sender<ShareJob>>,
    share_result_rx: Option<mpsc::Receiver<ShareResult>>,
}
```

**Modified StratumMetrics:**
```rust
pub struct StratumMetrics {
    // ... existing fields
    pub backpressure_total: AtomicU64,
    pub share_queue_depth: AtomicU64,
}
```

### Files Modified
- `crates/node/src/stratum_server.rs`: +268 lines, -65 lines
- `crates/tools/stress/src/main.rs`: Fixed unused variable warning

### Test Results
```
test result: ok. 31 passed; 0 failed; 0 ignored
```

All existing stratum tests pass:
- ✅ `miner_session_creation`
- ✅ `share_counters`
- ✅ `metrics_initialization`
- ✅ `metrics_recording`

### Behavior Changes
- **Before**: Share verification blocked async handler
- **After**: Share enqueued immediately, verified by worker pool off-reactor
- **Backpressure**: Queue full now returns explicit error instead of silent memory growth
- **Response**: Miner receives `accepted_for_verification: true` immediately (verification happens async)

### Performance Implications
- ✅ **Non-blocking submission**: Async handler never blocks on CPU work
- ✅ **Fair work distribution**: Workers contend fairly on shared receiver
- ✅ **Bounded memory**: Queue cap prevents OOM on burst loads
- ✅ **Graceful degradation**: Backpressure signal allows miner to throttle

### Metrics Available
```
stratum_backpressure_total
stratum_share_queue_depth
stratum_shares_accepted{algo="sha256d"}
stratum_shares_rejected{algo="sha256d",reason="..."}
```

### Next Steps (P2 COMMIT 3)
- Move miner hashing to `spawn_blocking` (prevent reactor starvation)
- Batch metrics flush (500ms tick) to reduce atomic contention

---

**Commit**: `be0d66b` - node/stratum: bounded share queue (1024), ShareVerifier worker pool (spawn_blocking), and backpressure metrics/error (-20001)
**Branch**: `fix/p2-stratum-bounded` → merged to `main`
**Date**: 2025-11-07
**Status**: ✅ COMPLETE, ALL TESTS PASS

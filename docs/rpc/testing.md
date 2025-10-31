# RPC Testing Guide

This note captures the conventions used by the BitQuan RPC crate when exercising
HTTP endpoints from async tests.

## Helper utilities

The module `crates/rpc/src/test_util.rs` exposes three helper functions:

- `spawn_test_server(server)` – binds `127.0.0.1:0`, launches the blocking
  `RpcServer` on a background thread, and returns `(base_url, join_handle,
  shutdown_tx)`. The handle should always be awaited (with a timeout) before the
  test exits.
- `wait_ready(base_url)` – polls `GET {base_url}/health` every 100 ms (client
  timeout 200 ms) for up to 5 s, ensuring the server is accepting requests.
- `init_test_tracing()` – installs a `tracing` formatter that writes to the test
  harness when `RUST_LOG` is enabled, making it easy to debug CI failures.

## Example usage

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rpc_without_auth_is_401() -> anyhow::Result<()> {
    init_test_tracing();
    let auth = RpcAuth::new("user", "pass");
    let server = RpcServer::with_auth(TestHandler, "127.0.0.1:0".into(), Some(auth));
    let (base_url, handle, shutdown_tx) = spawn_test_server(server)?;

    wait_ready(&base_url).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;

    let body = r#"{"jsonrpc":"2.0","method":"getblockcount","params":[],"id":1}"#;
    let response = tokio::time::timeout(
        Duration::from_secs(5),
        client
            .post(&base_url)
            .header("Content-Type", "application/json")
            .body(body)
            .send(),
    )
    .await??;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let _ = shutdown_tx.send(());
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await??;
    Ok(())
}
```

## Timeout conventions

- Every `reqwest::Client` is constructed with a 2 s request timeout.
- External awaits inside tests are wrapped with `tokio::time::timeout` (5 s) so
  failures surface quickly instead of hanging the suite.
- The server itself enforces read/write timeouts when parsing the HTTP request
  and responds with `Connection: close` to guarantee EOF for the client.

## Graceful shutdown

Tests should always:

1. Call `wait_ready` before sending authenticated requests.
2. Send on the returned `shutdown_tx` once assertions complete.
3. Await the server join handle (inside a 5 s timeout) to ensure all threads
   exit cleanly.

Following this pattern keeps the RPC tests deterministic and makes
troubleshooting in CI much easier.

## Configurable limits

The node exposes CLI flags (forwarded to `RpcConfig`) to tune runtime limits:

- `--rpc-max-body=<bytes>` (default 1_048_576)
- `--rpc-rl-burst=<tokens>` and `--rpc-rl-refill-per-sec=<tokens>` for the
  in-memory token bucket (per IP)
- `--rpc-conn-cooldown-ms=<millis>` to slow down abusive connections

Tests customise these values by constructing `RpcConfig` directly; production
operators can set the flags when launching `bitquan-node p2p-server`.

//! Test utilities for the RPC server.
//!
//! These helpers are compiled only for tests. They provide an async-friendly
//! wrapper around the blocking RPC server so integration-style tests can spawn
//! an instance on an ephemeral port, wait for readiness, and shut it down
//! cleanly without relying on `sleep` heuristics.

use crate::server::RpcServer;
use anyhow::Result;
use std::net::TcpListener;
use std::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

/// Spawns the RPC server on `127.0.0.1:0` and returns the base URL, task handle,
/// and shutdown sender.
pub(crate) fn spawn_test_server<T>(
    server: RpcServer<T>,
) -> Result<(String, JoinHandle<Result<()>>, oneshot::Sender<()>)>
where
    T: crate::methods::RpcMethods + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let (signal_tx, signal_rx) = mpsc::channel::<()>();

    tokio::spawn(async move {
        let _ = shutdown_rx.await;
        let _ = signal_tx.send(());
    });

    let handle = tokio::task::spawn_blocking(move || {
        server
            .serve_with_listener_and_shutdown(listener, Some(signal_rx))
            .map_err(|e| anyhow::anyhow!(e))
    });

    Ok((format!("http://{}", addr), handle, shutdown_tx))
}

/// Polls the `/health` endpoint until the server reports ready or the timeout
/// is exceeded.
pub(crate) async fn wait_ready(base_url: &str) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(200))
        .build()?;

    let health_url = format!("{}/health", base_url.trim_end_matches('/'));

    for _ in 0..50 {
        match client.get(&health_url).send().await {
            Ok(resp) if resp.status().is_success() => return Ok(()),
            Ok(_) | Err(_) => sleep(Duration::from_millis(100)).await,
        }
    }

    Err(anyhow::anyhow!("RPC server not ready after waiting 5s"))
}

/// Initialise pretty test logging once.
pub(crate) fn init_test_tracing() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    });
}

/// Convenience for building a basic-auth header in tests.
pub(crate) fn basic_auth_header(username: &str, password: &str) -> String {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    format!(
        "Basic {}",
        STANDARD.encode(format!("{}:{}", username, password))
    )
}

use bitquan_node::run_node;
use bitquan_types::NetworkId;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_node_starts_and_runs() {
    let p2p_bind = "127.0.0.1:0";
    let rpc_bind = "127.0.0.1:0";

    let node_handle = tokio::spawn(async move {
        run_node(
            "/dev/null",
            Some(rpc_bind),
            Some(p2p_bind),
            NetworkId::Devnet,
        )
        .await
    });

    sleep(Duration::from_millis(500)).await;

    // Verify node task is still running without early panic or crash
    assert!(
        !node_handle.is_finished(),
        "run_node terminated unexpectedly"
    );

    node_handle.abort();
}

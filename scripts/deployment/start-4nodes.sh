#!/bin/bash
# Start 4-node test network

echo "Building BitQuan node..."
cargo build --release --bin bitquan-node

echo "Starting Node 1 (seed)..."
./target/release/bitquan-node run --config config/mainnet.toml > node1.log 2>&1 &
NODE1_PID=$!
echo "Node 1 PID: $NODE1_PID"

sleep 2

echo "Starting Node 2..."
./target/release/bitquan-node run --config config/mainnet-node2.toml > node2.log 2>&1 &
NODE2_PID=$!
echo "Node 2 PID: $NODE2_PID"

sleep 2

echo "Starting Node 3..."
./target/release/bitquan-node run --config config/mainnet-node3.toml > node3.log 2>&1 &
NODE3_PID=$!
echo "Node 3 PID: $NODE3_PID"

sleep 2

echo "Starting Node 4..."
./target/release/bitquan-node run --config config/mainnet-node4.toml > node4.log 2>&1 &
NODE4_PID=$!
echo "Node 4 PID: $NODE4_PID"

echo ""
echo "All nodes started!"
echo "Node 1: PID $NODE1_PID, P2P 8333"
echo "Node 2: PID $NODE2_PID, P2P 8334"
echo "Node 3: PID $NODE3_PID, P2P 8335"
echo "Node 4: PID $NODE4_PID, P2P 8336"

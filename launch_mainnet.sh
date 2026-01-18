#!/bin/bash
# BitQuan Mainnet Launch Script
# Execute this to start the first node

echo "🚀 Starting BitQuan Mainnet Node..."
echo ""

./target/release/bitquan-node p2p-server \
    --datadir ./data/mainnet \
    --listen 0.0.0.0:18444 \
    --network mainnet \
    --rpc-listen 127.0.0.1:8332 \
    --rpc-username admin \
    --rpc-password mainnet_secure_2025 \
    --rpc-allow-insecure

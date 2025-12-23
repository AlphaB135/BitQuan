#!/bin/bash
set -e

# Kill any existing node
pkill -f bitquan-node || true

# Start node in background
echo "Starting node..."
./target/debug/bitquan-node p2p-server --rpc-listen 127.0.0.1:8332 --rpc-username admin --rpc-password secret --jwt-secret testsecret --rpc-allow-insecure > node.log 2>&1 &
NODE_PID=$!

# Wait for node to start
sleep 5

echo "Testing Valid Auth..."
curl -v -u admin:secret -X POST -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id": 1}' http://127.0.0.1:8332 > valid.log 2>&1
if grep -q "result" valid.log; then
    echo "✅ Valid Auth Passed"
else
    echo "❌ Valid Auth Failed"
    cat valid.log
    kill $NODE_PID
    exit 1
fi

echo "Testing Invalid Auth..."
curl -v -u admin:wrong -X POST -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id": 1}' http://127.0.0.1:8332 > invalid.log 2>&1
if grep -q "401 Unauthorized" invalid.log; then
    echo "✅ Invalid Auth Passed (Got 401)"
else
    echo "❌ Invalid Auth Failed (Did not get 401)"
    cat invalid.log
    kill $NODE_PID
    exit 1
fi

echo "Testing No Auth..."
curl -v -X POST -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id": 1}' http://127.0.0.1:8332 > noauth.log 2>&1
if grep -q "401 Unauthorized" noauth.log; then
    echo "✅ No Auth Passed (Got 401)"
else
    echo "❌ No Auth Failed (Did not get 401)"
    cat noauth.log
    kill $NODE_PID
    exit 1
fi

echo "🎉 All tests passed!"
kill $NODE_PID

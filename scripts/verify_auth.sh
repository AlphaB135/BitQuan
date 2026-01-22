#!/bin/bash
set -e

# ⚠️ SECURITY WARNING ⚠️
# These credentials are for TESTING ONLY and should NEVER be used in production.
# Override by setting environment variables before running this script.
#
# Example:
#   export TEST_RPC_PASSWORD="your_secure_password"
#   export TEST_JWT_SECRET="your_secure_jwt_secret"
#   ./scripts/verify_auth.sh

# Test credentials (can be overridden via environment)
TEST_RPC_USER="${TEST_RPC_USER:-admin}"
TEST_RPC_PASSWORD="${TEST_RPC_PASSWORD:-test_only_password_do_not_use_in_prod}"
TEST_JWT_SECRET="${TEST_JWT_SECRET:-test_only_jwt_secret_do_not_use_in_prod}"

# Kill any existing node
pkill -f bitquan-node || true

# Start node in background
echo "Starting node with TEST-ONLY credentials..."
./target/debug/bitquan-node p2p-server \
    --rpc-listen 127.0.0.1:8332 \
    --rpc-username "$TEST_RPC_USER" \
    --rpc-password "$TEST_RPC_PASSWORD" \
    --jwt-secret "$TEST_JWT_SECRET" \
    --rpc-allow-insecure > node.log 2>&1 &
NODE_PID=$!

# Wait for node to start
sleep 5

echo "Testing Valid Auth..."
curl -v -u "$TEST_RPC_USER:$TEST_RPC_PASSWORD" -X POST -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id": 1}' http://127.0.0.1:8332 > valid.log 2>&1
if grep -q "result" valid.log; then
    echo "✅ Valid Auth Passed"
else
    echo "❌ Valid Auth Failed"
    cat valid.log
    kill $NODE_PID
    exit 1
fi

echo "Testing Invalid Auth..."
curl -v -u "$TEST_RPC_USER:wrong_password" -X POST -H "Content-Type: application/json" -d '{"jsonrpc": "2.0", "method": "getblockcount", "params": [], "id": 1}' http://127.0.0.1:8332 > invalid.log 2>&1
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

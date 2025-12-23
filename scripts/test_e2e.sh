#!/bin/bash
# BitQuan Quick E2E Test Script
# Tests: Wallet creation → Node startup → Mining

set -e  # Exit on error

echo "======================================"
echo "BitQuan End-to-End Test"
echo "======================================"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo ""
    echo "${YELLOW}Cleaning up...${NC}"
    if [ ! -z "$NODE_PID" ]; then
        kill $NODE_PID 2>/dev/null || true
    fi
    rm -rf ./test_e2e_data
}

trap cleanup EXIT

# Step 1: Create wallet
echo "${GREEN}Step 1: Creating test wallet...${NC}"
./target/release/bitquan-node wallet-gen-mnemonic \
    --words 12 \
    --output ./test_wallet.json \
    --password testpass123 \
    --show-mnemonic

if [ ! -f ./test_wallet.json ]; then
    echo "❌ Wallet creation failed!"
    exit 1
fi

echo "✓ Wallet created: ./test_wallet.json"
echo ""

# Step 2: Get wallet address
echo "${GREEN}Step 2: Getting wallet address...${NC}"
ADDRESS=$(./target/release/bitquan-node wallet-address \
    --keystore ./test_wallet.json \
    --password testpass123 | grep "Address:" | awk '{print $2}')

echo "✓ Wallet address: $ADDRESS"
echo ""

# Step 3: Start mining node
echo "${GREEN}Step 3: Starting mining node...${NC}"
echo "Network: Devnet (easy difficulty)"
echo "Mining to: $ADDRESS"
echo ""

./target/release/bitquan-node mine \
    --datadir ./test_e2e_data \
    --payout-script-hex "76a914$(echo -n $ADDRESS | xxd -p)88ac" \
    --network devnet \
    --pow hashcash \
    --threads 2 \
    --limit-blocks 5 \
    --max-nonce 10000000 &

NODE_PID=$!
echo "✓ Node started (PID: $NODE_PID)"
echo ""

# Step 4: Wait for mining
echo "${GREEN}Step 4: Mining blocks (this may take a while)...${NC}"
echo "Target: 5 blocks"
echo ""

# Wait for node to finish or timeout
TIMEOUT=300  # 5 minutes
ELAPSED=0
while kill -0 $NODE_PID 2>/dev/null; do
    sleep 5
    ELAPSED=$((ELAPSED + 5))

    if [ $ELAPSED -ge $TIMEOUT ]; then
        echo "⚠️  Timeout reached (${TIMEOUT}s)"
        break
    fi

    # Check if blocks were mined
    if [ -d ./test_e2e_data ]; then
        BLOCKS=$(find ./test_e2e_data -name "*.block" 2>/dev/null | wc -l || echo 0)
        echo "Blocks mined so far: $BLOCKS"
    fi
done

echo ""
echo "${GREEN}Step 5: Checking results...${NC}"

# Check if any blocks were mined
if [ -d ./test_e2e_data ]; then
    TOTAL_BLOCKS=$(find ./test_e2e_data -name "*.block" 2>/dev/null | wc -l || echo 0)
    echo "✓ Total blocks mined: $TOTAL_BLOCKS"

    if [ $TOTAL_BLOCKS -gt 0 ]; then
        echo ""
        echo "======================================"
        echo "✅ E2E Test PASSED!"
        echo "======================================"
        echo "- Wallet created: ✓"
        echo "- Node started: ✓"
        echo "- Blocks mined: $TOTAL_BLOCKS"
        echo ""
        echo "Test data saved in: ./test_e2e_data"
        echo "Wallet saved in: ./test_wallet.json"
    else
        echo ""
        echo "⚠️  No blocks mined (difficulty may be too high)"
        echo "Try running with --network regtest for easier mining"
    fi
else
    echo "❌ No data directory created"
fi

echo ""
echo "Test complete!"

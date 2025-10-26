#!/bin/bash
# Genesis Block Generator for BitQuan
# This script creates the initial block for the blockchain

set -e

echo "========================================="
echo "BitQuan Genesis Block Generator"
echo "========================================="
echo ""

# Genesis parameters
GENESIS_TIME=${GENESIS_TIME:-$(date +%s)}
GENESIS_BITS=${GENESIS_BITS:-0x207fffff}  # Very easy initial difficulty
GENESIS_MESSAGE=${GENESIS_MESSAGE:-"The Quantum Age Begins - 26 Oct 2025. Ownerless. Verifiable. For everyone."}
GENESIS_REWARD=5000000000  # 50 BQ (in satoshis)

# Payout script (OP_RETURN for burn, or a test address)
# Default: OP_RETURN (no one can spend)
PAYOUT_SCRIPT=${PAYOUT_SCRIPT:-"6a"}  # OP_RETURN

echo "Genesis Parameters:"
echo "  Time: $GENESIS_TIME ($(date -r $GENESIS_TIME 2>/dev/null || date -d @$GENESIS_TIME 2>/dev/null))"
echo "  Bits: $GENESIS_BITS"
echo "  Message: $GENESIS_MESSAGE"
echo "  Reward: $GENESIS_REWARD qbits (50 BQ)"
echo "  Payout: $PAYOUT_SCRIPT"
echo ""

# Check if bitquan-node is built
if [ ! -f "./target/release/bitquan-node" ]; then
    echo "⚠️  bitquan-node not found. Building..."
    cargo build --release --features rocksdb-backend
fi

echo "🔨 Mining genesis block..."
echo "   This may take a while depending on difficulty..."
echo ""

# Create data directory
mkdir -p ./data/genesis

# Mine genesis block
./target/release/bitquan-node mine-once \
    --max-tries 10000000 \
    --payout-script-hex "$PAYOUT_SCRIPT" \
    --bits "$GENESIS_BITS"

echo ""
echo "✅ Genesis block generation complete!"
echo ""
echo "Next steps:"
echo "  1. Review the generated block"
echo "  2. Use this as the first block in your blockchain"
echo "  3. Start nodes with: ./target/release/bitquan-node run"
echo ""

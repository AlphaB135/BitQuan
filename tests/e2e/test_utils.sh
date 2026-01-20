#!/bin/bash
# E2E Test Utilities for BitQuan
# Helper functions for End-to-End testing

set -e

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test configuration
TEST_DATA_DIR="${TEST_DATA_DIR:-./test_e2e_data}"
MINER_WALLET="${MINER_WALLET:-./test_miner.keystore}"
RECEIVER_WALLET="${RECEIVER_WALLET:-./test_receiver.keystore}"
TEST_PASSWORD="${TEST_PASSWORD:-testpass123}"
NETWORK="${NETWORK:-regtest}"
BITQUAN_NODE="${BITQUAN_NODE:-cargo run --quiet -p bitquan-node --}"

# Cleanup functions
cleanup_test_environment() {
    echo -e "${BLUE}Cleaning up test environment...${NC}"
    # Kill any running bitquan-node processes
    pkill -f "bitquan-node" 2>/dev/null || true
    sleep 1

    # Remove test data directory
    if [ -d "$TEST_DATA_DIR" ]; then
        rm -rf "$TEST_DATA_DIR"
    fi

    # Remove test wallet files
    rm -f "$MINER_WALLET" "$RECEIVER_WALLET" test_*.keystore
    rm -f data/pending_transactions.jsonl

    echo -e "${GREEN}✓ Cleanup complete${NC}"
}

setup_test_environment() {
    echo -e "${BLUE}Setting up test environment...${NC}"

    # Create test data directory
    mkdir -p "$TEST_DATA_DIR"
    mkdir -p "$TEST_DATA_DIR/blocks"

    echo -e "${GREEN}✓ Test environment ready${NC}"
}

# Mining helper
mine_blocks() {
    local count=$1
    local script_hex=$2

    echo -e "${BLUE}Mining $count blocks...${NC}"

    # Use hashcash (easiest) for regtest
    # Capture output to verify success
    local output=$($BITQUAN_NODE mine \
        --datadir "$TEST_DATA_DIR" \
        --payout-script-hex "$script_hex" \
        --network "$NETWORK" \
        --pow hashcash \
        --limit-blocks "$count" \
        --max-nonce 1000000 \
        2>&1 | tee /tmp/mine_output.log)

    echo "$output"

    # Check if mining succeeded
    if echo "$output" | grep -q "Session complete\|Total: 101"; then
        echo -e "${GREEN}✓ Mined $count blocks${NC}"
    else
        echo -e "${RED}✗ Mining may have failed${NC}"
        return 1
    fi
}

# Wallet generation
generate_wallet() {
    local wallet_path=$1
    local name=$2

    echo -e "${BLUE}Generating $name wallet...${NC}"

    $BITQUAN_NODE wallet-gen-mnemonic \
        --words 12 \
        --output "$wallet_path" \
        --password "$TEST_PASSWORD" \
        > /dev/null 2>&1

    if [ ! -f "$wallet_path" ]; then
        echo -e "${RED}✗ Failed to create wallet${NC}"
        return 1
    fi

    echo -e "${GREEN}✓ $name wallet created${NC}"
}

# Get address from wallet
get_wallet_address() {
    local wallet_path=$1

    # Run command and capture all output
    local output=$($BITQUAN_NODE wallet-address \
        --keystore "$wallet_path" \
        --password "$TEST_PASSWORD" 2>&1)

    # Extract address from output (format: "Address: bq1...")
    # Try sed approach (portable across macOS and Linux)
    echo "$output" | sed -n 's/.*Address: \([a-z0-9]*\).*/\1/p'
}

# Convert address to script hex for mining
get_script_from_address() {
    local address=$1

    # script-from-address outputs hex with trailing whitespace
    # Use xargs to trim whitespace
    $BITQUAN_NODE script-from-address --address "$address" 2>/dev/null | xargs
}

# Get balance (in qbits)
get_balance() {
    local address=$1

    # Get just the qbits value from first "Balance:" line
    # Format: "Balance: 5050000000000000000000 qbits"
    $BITQUAN_NODE balance \
        --datadir "$TEST_DATA_DIR" \
        --address "$address" 2>/dev/null | \
        grep "Balance:" | \
        head -1 | \
        awk '{print $2}' | \
        tr -d ','
}

# Convert BQ to qbits (1 BQ = 10^18 qbits) using bc for big numbers
bq_to_qbits() {
    local bq=$1
    echo "$1 * 1000000000000000000" | bc
}

# Convert qbits to BQ
qbits_to_bq() {
    local qbits=$1
    echo "scale=18; $qbits / 1000000000000000000" | bc
}

# Verification helpers (use bc for big number comparison)
assert_not_zero() {
    local value=$1
    local msg=$2

    if [ "$value" = "0" ]; then
        echo -e "${RED}✗ FAILED: $msg (value is zero)${NC}"
        return 1
    fi
    echo -e "${GREEN}✓ $msg: $value${NC}"
}

assert_equals() {
    local expected=$1
    local actual=$2
    local msg=$3

    if [ "$expected" != "$actual" ]; then
        echo -e "${RED}✗ FAILED: $msg (expected: $expected, got: $actual)${NC}"
        return 1
    fi
    echo -e "${GREEN}✓ $msg${NC}"
}

assert_greater_than() {
    local min=$1
    local actual=$2
    local msg=$3

    # Use bc for comparison: returns 1 if actual > min, 0 otherwise
    local result=$(echo "$actual > $min" | bc)
    if [ "$result" = "0" ]; then
        echo -e "${RED}✗ FAILED: $msg (expected > $min, got: $actual)${NC}"
        return 1
    fi
    echo -e "${GREEN}✓ $msg${NC}"
}

# Arithmetic helpers for big numbers
bc_lt() {
    # Returns 1 if $1 < $2, 0 otherwise
    echo "$1 < $2" | bc
}

bc_gt() {
    # Returns 1 if $1 > $2, 0 otherwise
    echo "$1 > $2" | bc
}

bc_sub() {
    # Returns $1 - $2
    echo "$1 - $2" | bc
}

bc_add() {
    # Returns $1 + $2
    echo "$1 + $2" | bc
}

# Get blockchain height
get_chain_height() {
    # Check chainstate RocksDB for block count
    # The mine command outputs final height, let's parse from logs or use RPC
    # For now, use a file-based approach - mine creates a chainstate directory

    # Try to count from data directory structure
    if [ -d "$TEST_DATA_DIR/chainstate" ]; then
        # Use bitquan-node to get height if available, or count database files
        # For now, return file count as approximation
        ls "$TEST_DATA_DIR/chainstate" 2>/dev/null | wc -l | tr -d ' '
    else
        echo "0"
    fi
}

# Check if transaction is in pending file
transaction_pending() {
    [ -f "data/pending_transactions.jsonl" ] && [ -s "data/pending_transactions.jsonl" ]
}

# Print section header
print_section() {
    local title=$1
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}  $title${NC}"
    echo -e "${BLUE}═════════════════════════════════════════════════════${NC}"
}

# Print success
print_success() {
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║           ✅ TEST PASSED ✅                      ║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════╝${NC}"
}

# Print failure
print_failure() {
    local msg=$1
    echo ""
    echo -e "${RED}╔═══════════════════════════════════════════════════╗${NC}"
    echo -e "${RED}║           ❌ TEST FAILED ❌                      ║${NC}"
    echo -e "${RED}╚═══════════════════════════════════════════════════╝${NC}"
    echo -e "${RED}Error: $msg${NC}"
}

# Trap errors
trap 'print_failure "Test script failed at line $LINENO"; cleanup_test_environment; exit 1' ERR

# Export functions for use in subshells
export -f cleanup_test_environment
export -f setup_test_environment
export -f mine_blocks
export -f generate_wallet
export -f get_wallet_address
export -f get_script_from_address
export -f get_balance
export -f assert_not_zero
export -f assert_equals
export -f assert_greater_than
export -f bc_lt
export -f bc_gt
export -f bc_sub
export -f bc_add
export -f get_chain_height
export -f transaction_pending
export -f print_section
export -f print_success
export -f print_failure

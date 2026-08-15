#!/bin/bash
# Automated defense responses for BitQuan
# Created by: Hermes (ซากุระ) 🌸

set -euo pipefail

# Configuration
BITQUAN_CLI="${BITQUAN_CLI:-/home/ubuntu/bitquan-audit/target/release/bitquan-cli}"
FIREWALL_RULES="/etc/bitquan/firewall.rules"
MAX_MEMPOOL_SIZE=50000
MIN_PEER_DIVERSITY=5
CHECK_INTERVAL=30

# Colors
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Create directories
mkdir -p /etc/bitquan
touch "$FIREWALL_RULES"

echo -e "${GREEN}🛡️  BitQuan Auto-Defense System — Started at $(date)${NC}"
echo ""

# Function: Ban IP permanently
ban_ip() {
    local IP=$1
    local REASON=$2

    echo -e "${RED}🚫 Banning IP: $IP${NC}"
    echo "Reason: $REASON"

    # Add to iptables (requires sudo)
    sudo iptables -A INPUT -s "$IP" -j DROP 2>/dev/null || \
        echo "Warning: Could not add iptables rule (need sudo)"

    # Persist to firewall rules
    echo "$(date +%Y-%m-%d\ %H:%M:%S) $IP # $REASON" >> "$FIREWALL_RULES"
}

# Function: Rate limit IP
rate_limit_ip() {
    local IP=$1
    local RATE=${2:-10}  # packets per second

    echo -e "${YELLOW}⚠️  Rate limiting $IP to $RATE pps${NC}"

    sudo iptables -A INPUT -s "$IP" -m limit --limit "$RATE/s" -j ACCEPT 2>/dev/null
    sudo iptables -A INPUT -s "$IP" -j DROP 2>/dev/null
}

# Function: Check if BitQuan CLI is available
check_cli() {
    if [ ! -f "$BITQUAN_CLI" ]; then
        echo -e "${RED}Error: bitquan-cli not found at $BITQUAN_CLI${NC}"
        return 1
    fi
    return 0
}

# Function: Monitor mempool for spam
check_mempool_spam() {
    if ! check_cli; then
        return 1
    fi

    local MEMPOOL_SIZE=$($BITQUAN_CLI getrawmempool 2>/dev/null | jq 'length' 2>/dev/null || echo 0)

    if [ "$MEMPOOL_SIZE" -gt "$MAX_MEMPOOL_SIZE" ]; then
        echo -e "${RED}🚨 Mempool spam detected: $MEMPOOL_SIZE transactions${NC}"
        echo "Max threshold: $MAX_MEMPOOL_SIZE"

        # Get low-fee transactions
        echo "Analyzing transaction fees..."

        # Alert but don't auto-clear (requires manual decision)
        echo -e "${YELLOW}⚠️  Manual intervention needed: Review mempool and clear low-fee txs${NC}"
        echo "Command: $BITQUAN_CLI clearmempool --min-fee=0.0001"

        return 0
    else
        echo -e "${GREEN}✓ Mempool size OK: $MEMPOOL_SIZE transactions${NC}"
        return 1
    fi
}

# Function: Check peer diversity (Eclipse attack detection)
check_peer_diversity() {
    if ! check_cli; then
        return 1
    fi

    local PEER_INFO=$($BITQUAN_CLI getpeerinfo 2>/dev/null || echo '[]')
    local PEER_COUNT=$(echo "$PEER_INFO" | jq 'length' 2>/dev/null || echo 0)

    if [ "$PEER_COUNT" -eq 0 ]; then
        echo -e "${RED}🚨 No peers connected!${NC}"
        return 0
    fi

    # Extract peer IPs and count unique /24 subnets
    local UNIQUE_SUBNETS=$(echo "$PEER_INFO" | \
        jq -r '.[].addr' 2>/dev/null | \
        grep -oP '^\d+\.\d+\.\d+' | \
        sort -u | \
        wc -l || echo 0)

    echo -e "${BLUE}📊 Peer stats: $PEER_COUNT peers from $UNIQUE_SUBNETS unique subnets${NC}"

    if [ "$UNIQUE_SUBNETS" -lt "$MIN_PEER_DIVERSITY" ]; then
        echo -e "${RED}🚨 Low peer diversity detected!${NC}"
        echo "Unique subnets: $UNIQUE_SUBNETS (threshold: $MIN_PEER_DIVERSITY)"
        echo "Possible Eclipse attack!"

        # Find majority subnet
        local MAJORITY_SUBNET=$(echo "$PEER_INFO" | \
            jq -r '.[].addr' 2>/dev/null | \
            grep -oP '^\d+\.\d+\.\d+' | \
            sort | uniq -c | sort -rn | head -1 | awk '{print $2}' || echo "")

        if [ ! -z "$MAJORITY_SUBNET" ]; then
            echo "Majority subnet: $MAJORITY_SUBNET"
            echo -e "${YELLOW}⚠️  Consider disconnecting peers from $MAJORITY_SUBNET${NC}"

            # List peers to disconnect
            echo "$PEER_INFO" | \
                jq -r ".[] | select(.addr | startswith(\"$MAJORITY_SUBNET\")) | .addr" 2>/dev/null | \
            while read peer; do
                echo "  - Suspicious peer: $peer"
            done
        fi

        return 0
    else
        echo -e "${GREEN}✓ Peer diversity OK${NC}"
        return 1
    fi
}

# Function: Check block production (51% attack / mining issues)
check_block_production() {
    if ! check_cli; then
        return 1
    fi

    local CURRENT_HEIGHT=$($BITQUAN_CLI getblockcount 2>/dev/null || echo 0)
    local BEST_HASH=$($BITQUAN_CLI getbestblockhash 2>/dev/null || echo "")

    if [ -z "$BEST_HASH" ]; then
        echo -e "${RED}🚨 Cannot get best block hash!${NC}"
        return 0
    fi

    local BLOCK_INFO=$($BITQUAN_CLI getblock "$BEST_HASH" 2>/dev/null || echo '{}')
    local BLOCK_TIME=$(echo "$BLOCK_INFO" | jq -r '.time' 2>/dev/null || echo 0)
    local NOW=$(date +%s)
    local TIME_SINCE_BLOCK=$((NOW - BLOCK_TIME))

    # BitQuan target: 120 seconds
    # Alert if > 600 seconds (5 blocks missed)
    if [ "$TIME_SINCE_BLOCK" -gt 600 ]; then
        echo -e "${RED}🚨 No new blocks for $TIME_SINCE_BLOCK seconds!${NC}"
        echo "Expected: ~120 seconds per block"
        echo "Current height: $CURRENT_HEIGHT"
        echo "Possible issues: Network partition, 51% attack, miner crash"
        return 0
    else
        echo -e "${GREEN}✓ Block production OK (last block $TIME_SINCE_BLOCK sec ago)${NC}"
        return 1
    fi
}

# Function: Check RPC endpoint health
check_rpc_health() {
    local RPC_ENDPOINT="${RPC_ENDPOINT:-http://127.0.0.1:19443}"

    local RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$RPC_ENDPOINT" 2>/dev/null || echo "000")

    if [ "$RESPONSE" = "000" ]; then
        echo -e "${RED}🚨 RPC endpoint not responding!${NC}"
        return 0
    elif [ "$RESPONSE" = "401" ]; then
        echo -e "${GREEN}✓ RPC endpoint healthy (auth required)${NC}"
        return 1
    elif [ "$RESPONSE" = "200" ]; then
        echo -e "${YELLOW}⚠️  RPC endpoint responding without auth!${NC}"
        return 0
    else
        echo -e "${YELLOW}⚠️  RPC returned unexpected code: $RESPONSE${NC}"
        return 0
    fi
}

# Function: Check system resources
check_system_resources() {
    # Check disk space
    local DISK_USAGE=$(df -h / | awk 'NR==2 {print $5}' | sed 's/%//')

    if [ "$DISK_USAGE" -gt 90 ]; then
        echo -e "${RED}🚨 Disk usage critical: ${DISK_USAGE}%${NC}"
        echo "Consider enabling pruning or expanding disk"
    elif [ "$DISK_USAGE" -gt 80 ]; then
        echo -e "${YELLOW}⚠️  Disk usage high: ${DISK_USAGE}%${NC}"
    else
        echo -e "${GREEN}✓ Disk usage OK: ${DISK_USAGE}%${NC}"
    fi

    # Check memory
    local MEM_USAGE=$(free | awk 'NR==2 {printf "%.0f", $3/$2 * 100}')

    if [ "$MEM_USAGE" -gt 90 ]; then
        echo -e "${RED}🚨 Memory usage critical: ${MEM_USAGE}%${NC}"
    elif [ "$MEM_USAGE" -gt 80 ]; then
        echo -e "${YELLOW}⚠️  Memory usage high: ${MEM_USAGE}%${NC}"
    else
        echo -e "${GREEN}✓ Memory usage OK: ${MEM_USAGE}%${NC}"
    fi

    # Check CPU load
    local CPU_LOAD=$(uptime | awk -F'load average:' '{print $2}' | awk '{print $1}' | sed 's/,//')
    local CPU_CORES=$(nproc)

    echo -e "${BLUE}📊 CPU load: $CPU_LOAD (cores: $CPU_CORES)${NC}"
}

# Function: Generate security report
generate_report() {
    local REPORT_FILE="/tmp/bitquan_security_report_$(date +%Y%m%d_%H%M%S).txt"

    echo "BitQuan Security Report" > "$REPORT_FILE"
    echo "Generated: $(date)" >> "$REPORT_FILE"
    echo "================================" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    echo "Banned IPs:" >> "$REPORT_FILE"
    cat "$FIREWALL_RULES" >> "$REPORT_FILE" 2>/dev/null || echo "None" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    if check_cli; then
        echo "Blockchain Status:" >> "$REPORT_FILE"
        $BITQUAN_CLI getblockchaininfo >> "$REPORT_FILE" 2>/dev/null || echo "Error" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"

        echo "Network Status:" >> "$REPORT_FILE"
        $BITQUAN_CLI getnetworkinfo >> "$REPORT_FILE" 2>/dev/null || echo "Error" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
    fi

    echo -e "${GREEN}📄 Security report saved: $REPORT_FILE${NC}"
}

# Main monitoring loop
echo "Starting continuous monitoring (interval: ${CHECK_INTERVAL}s)"
echo ""

ITERATION=0

while true; do
    ITERATION=$((ITERATION + 1))
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}Iteration #$ITERATION — $(date)${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo ""

    # Run checks
    check_rpc_health
    echo ""

    check_mempool_spam
    echo ""

    check_peer_diversity
    echo ""

    check_block_production
    echo ""

    check_system_resources
    echo ""

    # Generate report every 10 iterations (~5 minutes if interval=30s)
    if [ $((ITERATION % 10)) -eq 0 ]; then
        generate_report
        echo ""
    fi

    echo -e "${BLUE}Next check in ${CHECK_INTERVAL} seconds...${NC}"
    echo ""

    sleep "$CHECK_INTERVAL"
done

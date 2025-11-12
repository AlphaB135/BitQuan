#!/usr/bin/env bash
# BitQuan Testnet Auto Manager for macOS
# One-command setup and management for testnet nodes on macOS

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration for macOS
BITQUAN_USER=$(whoami)
INSTALL_DIR="$HOME/bitquan-testnet"
DATA_DIR="$INSTALL_DIR/data/testnet"
CONFIG_DIR="$INSTALL_DIR/config"
SERVICE_NAME="bitquan-testnet"
PID_FILE="$INSTALL_DIR/bitquan.pid"
MINING_PID_FILE="$INSTALL_DIR/mining.pid"
RPC_PID_FILE="$INSTALL_DIR/rpc.pid"

# Function to display banner
show_banner() {
    clear
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║      BitQuan Testnet Auto Manager v2.0.0 (macOS)            ║"
    echo "║      จัดการโหนด Testnet อัตโนมัติทั้งหมดในคำสั่งเดียว        ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    # Show installation status
    echo -e "${YELLOW}สถานะการติดตั้ง:${NC}"
    if [[ -f "$INSTALL_DIR/bin/bitquan-node" ]] && [[ -f "$INSTALL_DIR/start.sh" ]]; then
        echo -e "  การติดตั้ง: ${GREEN}ติดตั้งแล้ว${NC}"
        echo -e "  ไดเรกทอรี: $INSTALL_DIR"
        
        # Show node status
        if [[ -f "$PID_FILE" ]] && kill -0 $(cat "$PID_FILE") 2>/dev/null; then
            echo -e "  สถานะโหนด: ${GREEN}กำลังทำงาน${NC}"
            echo -e "  PID: $(cat "$PID_FILE")"
            
        # Show network info
        echo -e "  P2P Port: $(lsof -i :19444 2>/dev/null | grep LISTEN | wc -l | tr -d ' ') connection(s)"
        echo -e "  RPC Port: $(lsof -i :19443 2>/dev/null | grep LISTEN | wc -l | tr -d ' ') connection(s)"
        
        # Show RPC status
        if curl -s --connect-timeout 2 http://localhost:19443/health >/dev/null 2>&1; then
            echo -e "  RPC Server: ${GREEN}ทำงาน${NC}"
        else
            echo -e "  RPC Server: ${RED}ไม่ทำงาน${NC}"
        fi
        
        # Show mining status
        if [[ -f "$MINING_PID_FILE" ]] && kill -0 $(cat "$MINING_PID_FILE") 2>/dev/null; then
            echo -e "  การขุด: ${GREEN}กำลังขุด${NC} (PID: $(cat "$MINING_PID_FILE"))"
        else
            echo -e "  การขุด: ${RED}ไม่ได้ขุด${NC}"
        fi
        else
            echo -e "  สถานะโหนด: ${RED}หยุดทำงาน${NC}"
        fi
        
        # Show wallet info
        if [[ -f "$CONFIG_DIR/pool-wallet.keystore" ]]; then
            echo -e "  Wallet: ${GREEN}มี wallet${NC}"
        else
            echo -e "  Wallet: ${RED}ไม่มี wallet${NC}"
        fi
        
        # Show data size
        if [[ -d "$DATA_DIR" ]]; then
            DATA_SIZE=$(du -sh "$DATA_DIR" 2>/dev/null | cut -f1 || echo "0B")
            echo -e "  ขนาดข้อมูล: $DATA_SIZE"
        fi
        
    else
        echo -e "  การติดตั้ง: ${RED}ยังไม่ได้ติดตั้ง${NC}"
        echo -e "  ไดเรกทอรี: $INSTALL_DIR (ยังไม่มี)"
    fi
    echo ""
}

# Function to check if running on macOS
check_macos() {
    if [[ "$(uname)" != "Darwin" ]]; then
        echo -e "${RED}❌ สคริปนี้สำหรับ macOS เท่านั้น${NC}"
        exit 1
    fi
}

# Function to install dependencies for macOS
install_dependencies() {
    echo -e "${YELLOW}📦 กำลังติดตั้ง dependencies...${NC}"
    
    # Check if Homebrew is installed
    if ! command -v brew &> /dev/null; then
        echo -e "${YELLOW}🍺 กำลังติดตั้ง Homebrew...${NC}"
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
        
        # Add Homebrew to PATH for Apple Silicon Macs
        if [[ $(uname -m) == "arm64" ]]; then
            echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zshrc
            eval "$(/opt/homebrew/bin/brew shellenv)"
        fi
    fi
    
    # Install dependencies
    echo -e "${YELLOW}📦 กำลังติดตั้ง packages...${NC}"
    brew install openssl jq rust git
    
    echo -e "${GREEN}✅ ติดตั้ง dependencies เรียบร้อย${NC}"
}

# Function to create directories
setup_directories() {
    echo -e "${YELLOW}📁 กำลังสร้าง directories...${NC}"
    
    # Create directories
    mkdir -p "$INSTALL_DIR"/{bin,data,logs,backups,config}
    mkdir -p "$DATA_DIR"
    mkdir -p "$CONFIG_DIR"
    mkdir -p "$INSTALL_DIR/logs"
    
    echo "✅ สร้าง directories เรียบร้อย"
}

# Function to get BitQuan binary
get_binary() {
    echo -e "${YELLOW}⬇️ กำลังดาวน์โหลด/ build BitQuan binary...${NC}"
    
    # Try to download from releases first
    BIN_URL="https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-darwin-x86_64"
    if curl -fsSL "$BIN_URL" -o "$INSTALL_DIR/bin/bitquan-node" 2>/dev/null; then
        echo "✅ ดาวน์โหลดจาก GitHub releases"
    else
        echo "⚠️ ไม่พบ release กำลัง build จาก source..."
        
        # Install Rust if not installed
        if ! command -v cargo &> /dev/null; then
            echo "📦 กำลังติดตั้ง Rust..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
            source "$HOME/.cargo/env"
        fi
        
        # Clone and build
        cd /tmp
        if [[ -d BitQuan ]]; then
            rm -rf BitQuan
        fi
        git clone https://github.com/AlphaB135/BitQuan.git BitQuan
        cd BitQuan
        git checkout v1.0.0 || git checkout main
        cargo build --release --bin bitquan-node
        cp target/release/bitquan-node "$INSTALL_DIR/bin/"
        cd /tmp
        rm -rf BitQuan
        echo "✅ Build จาก source เรียบร้อย"
    fi
    
    chmod +x "$INSTALL_DIR/bin/bitquan-node"
    echo "✅ ติดตั้ง binary เรียบร้อย"
}

# Function to generate JWT secret
generate_jwt_secret() {
    echo -e "${YELLOW}🔐 กำลังสร้าง JWT secret...${NC}"
    JWT_SECRET=$(openssl rand -hex 32)
    echo "$JWT_SECRET" > "$CONFIG_DIR/jwt.secret"
    chmod 600 "$CONFIG_DIR/jwt.secret"
    echo "✅ สร้าง JWT secret เรียบร้อย"
}

# Function to create wallet
create_wallet() {
    echo -e "${YELLOW}💰 กำลังสร้าง testnet wallet...${NC}"
    
    # Ask for wallet password
    while true; do
        read -s -p "🔑 ใส่รหัสผ่านสำหรับ wallet (อย่างน้อย 8 ตัวอักษร): " WALLET_PASSWORD
        echo
        if [[ ${#WALLET_PASSWORD} -lt 8 ]]; then
            echo -e "${RED}❌ รหัสผ่านต้องมีอย่างน้อย 8 ตัวอักษร${NC}"
            continue
        fi
        read -s -p "🔑 ยืนยันรหัสผ่าน: " WALLET_PASSWORD_CONFIRM
        echo
        if [[ "$WALLET_PASSWORD" == "$WALLET_PASSWORD_CONFIRM" ]]; then
            break
        else
            echo -e "${RED}❌ รหัสผ่านไม่ตรงกัน กรุณาลองใหม่${NC}"
        fi
    done
    
    # Create wallet
    "$INSTALL_DIR/bin/bitquan-node" wallet-gen \
      --network testnet \
      --output "$CONFIG_DIR/pool-wallet.keystore" \
      --password "$WALLET_PASSWORD" || true
    
    # Get mining address
    MINING_ADDRESS=$("$INSTALL_DIR/bin/bitquan-node" wallet-address \
      --keystore "$CONFIG_DIR/pool-wallet.keystore" \
      --password "$WALLET_PASSWORD" 2>/dev/null | grep "📍 Address:" | cut -d' ' -f3 || echo "tBQ1_CHANGE_ME")
    echo "✅ Mining address: $MINING_ADDRESS"
}

# Function to create configuration
create_config() {
    echo -e "${YELLOW}⚙️ กำลังสร้าง configuration...${NC}"
    
    cat > "$CONFIG_DIR/testnet.toml" << EOF
# BitQuan Testnet Configuration for macOS

[network]
id = "testnet"
p2p_port = 19444
rpc_port = 19443
bootstrap_nodes = [
    "node1.bitquan.dev:19444",
    "node2.bitquan.dev:19444",
]
difficulty_bits = "0x1d00ffff"
block_interval_seconds = 600

[consensus]
asert_half_life = 172800
burst_guard_enabled = true
burst_guard_threshold = 1.5
max_block_weight = 4000000

[mempool]
max_size_bytes = 104857600
min_relay_fee_per_wu = 1
max_tx_size = 100000

[rpc]
bind = "127.0.0.1:19443"
auth_enabled = true
jwt_secret_file = "$CONFIG_DIR/jwt.secret"

[mining]
coinbase_maturity = 100
initial_block_reward = 5000000000
halving_interval = 210000
address = "$MINING_ADDRESS"

[wallet]
keystore_kdf_mem_kib = 65536
keystore_kdf_time_cost = 3
keystore_kdf_parallelism = 1

[logging]
level = "info"
log_file = "$DATA_DIR/node.log"
max_log_size_mb = 100
max_log_files = 10

[storage]
db_path = "$DATA_DIR/chainstate"
cache_size_mb = 512

[network.limits]
max_inbound_peers = 125
max_outbound_peers = 8
max_message_size = 33554432
max_upload_bytes_per_sec = 1048576
max_download_bytes_per_sec = 5242880

[testnet]
allow_mining_without_peers = true
fast_sync_enabled = true
checkpoint_enabled = false
faucet_url = "https://faucet.bitquan.dev"
explorer_url = "https://explorer.bitquan.dev"
reset_interval_blocks = 0
EOF

    echo "✅ สร้าง configuration เรียบร้อย"
}

# Function to create helper scripts
create_helper_scripts() {
    echo -e "${YELLOW}📝 กำลังสร้าง helper scripts...${NC}"
    
    # Start script
    cat > "$INSTALL_DIR/start.sh" << EOF
#!/bin/bash
# Start BitQuan Testnet Node

cd "$INSTALL_DIR"
if [[ -f "$PID_FILE" ]]; then
    if kill -0 \$(cat "$PID_FILE") 2>/dev/null; then
        echo "❌ BitQuan node กำลังทำงานอยู่แล้ว"
        exit 1
    else
        rm -f "$PID_FILE"
    fi
fi

# Generate RPC certificate if not exists
if [[ ! -f "$CONFIG_DIR/cert.pem" ]]; then
    echo "🔐 กำลังสร้าง RPC certificate..."
    "$INSTALL_DIR/bin/bitquan-node" generate-cert --output "$CONFIG_DIR"
fi

echo "🚀 กำลังเปิด BitQuan testnet node..."
nohup "$INSTALL_DIR/bin/bitquan-node" run \\
  --config "$CONFIG_DIR/testnet.toml" \\
  > "$INSTALL_DIR/logs/testnet.log" 2>&1 &

NODE_PID=\$!
echo \$NODE_PID > "$PID_FILE"

sleep 3

if kill -0 \$(cat "$PID_FILE") 2>/dev/null; then
    echo "✅ BitQuan testnet node เริ่มทำงานแล้ว"
    echo "Node PID: \$(cat "$PID_FILE")"
    echo "Note: RPC server อาจต้องเปิดแยกต่างหาก"
else
    echo "❌ ไม่สามารถเปิดโหนดได้"
    rm -f "$PID_FILE"
    exit 1
fi
EOF

    # Stop script
    cat > "$INSTALL_DIR/stop.sh" << EOF
#!/bin/bash
# Stop BitQuan Testnet Node

# RPC server is part of main node, no separate process to stop

# Stop main node
if [[ ! -f "$PID_FILE" ]]; then
    echo "❌ ไม่พบ PID file โหนดอาจไม่ทำงาน"
    exit 1
fi

PID=\$(cat "$PID_FILE")
if kill -0 "\$PID" 2>/dev/null; then
    echo "🛑 กำลังปิด BitQuan testnet node (PID: \$PID)..."
    kill "\$PID"
    
    # Wait for graceful shutdown
    for i in {1..10}; do
        if ! kill -0 "\$PID" 2>/dev/null; then
            break
        fi
        sleep 1
    done
    
    # Force kill if still running
    if kill -0 "\$PID" 2>/dev/null; then
        echo "⚠️ บังคับปิดโหนด..."
        kill -9 "\$PID"
    fi
    
    rm -f "$PID_FILE"
    echo "✅ ปิดโหนดเรียบร้อย"
else
    echo "❌ โหนดไม่ทำงาน"
    rm -f "$PID_FILE"
fi
EOF

    # Status script
    cat > "$INSTALL_DIR/status.sh" << EOF
#!/bin/bash
# Check BitQuan Testnet Node Status

echo "=== BitQuan Testnet Node Status ==="
echo ""

if [[ -f "$PID_FILE" ]]; then
    PID=\$(cat "$PID_FILE")
    if kill -0 "\$PID" 2>/dev/null; then
        echo "Status: ${GREEN}● กำลังทำงาน${NC}"
        echo "PID: \$PID"
        echo ""
        echo "=== Network Ports ==="
        lsof -i :19443 2>/dev/null || echo "RPC port 19443 ไม่เปิด"
        lsof -i :19444 2>/dev/null || echo "P2P port 19444 ไม่เปิด"
        echo ""
        echo "=== Recent Logs ==="
        tail -10 "$INSTALL_DIR/logs/testnet.log" 2>/dev/null || echo "ไม่พบ logs"
    else
        echo "Status: ${RED}● หยุดทำงาน (PID file มีอยู่แต่ process ไม่ทำงาน)${NC}"
        rm -f "$PID_FILE"
    fi
else
    echo "Status: ${RED}● หยุดทำงาน${NC}"
fi

echo ""
echo "=== Configuration ==="
echo "Data Dir: $DATA_DIR"
echo "Config: $CONFIG_DIR/testnet.toml"
echo "Logs: $INSTALL_DIR/logs/testnet.log"
EOF

    # Logs script
    cat > "$INSTALL_DIR/logs.sh" << EOF
#!/bin/bash
# View BitQuan Testnet Node Logs

if [[ -f "$INSTALL_DIR/logs/testnet.log" ]]; then
    echo "กำลังแสดง logs (กด Ctrl+C เพื่อออก):"
    tail -f "$INSTALL_DIR/logs/testnet.log"
else
    echo "❌ ไม่พบ log file"
fi
EOF

    # Menu script
    cat > "$INSTALL_DIR/menu.sh" << 'EOF'
#!/bin/bash
# BitQuan Testnet Management Menu for macOS

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

INSTALL_DIR="$HOME/bitquan-testnet"
PID_FILE="$INSTALL_DIR/bitquan.pid"

show_menu() {
    clear
    echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║           BitQuan Testnet Management Menu (macOS)          ║${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}📊 สถานะโหนด:${NC}"
    if [[ -f "$PID_FILE" ]] && kill -0 $(cat "$PID_FILE") 2>/dev/null; then
        echo -e "  Status: ${GREEN}● กำลังทำงาน${NC}"
    else
        echo -e "  Status: ${RED}● หยุดทำงาน${NC}"
    fi
    echo ""
    echo -e "${YELLOW}🛠️  เมนู:${NC}"
    echo "  1) เปิดโหนด (Start Node)"
    echo "  2) ปิดโหนด (Stop Node)"
    echo "  3) รีสตาร์ทโหนด (Restart Node)"
    echo "  4) ดูสถานะ (View Status)"
    echo "  5) ดู logs (View Logs)"
    echo "  6) ดูข้อมูล blockchain (Blockchain Info)"
    echo "  7) สร้างที่อยู่ใหม่ (Create New Address)"
    echo "  8) ดู wallet balance (Check Balance)"
    echo "  9) ออกจากเมนู (Exit)"
    echo ""
    read -p "เลือกตัวเลือก (1-9): " choice
}

handle_choice() {
    case $choice in
        1)
            "$INSTALL_DIR/start.sh"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        2)
            "$INSTALL_DIR/stop.sh"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        3)
            echo -e "${YELLOW}🔄 กำลังรีสตาร์ทโหนด...${NC}"
            "$INSTALL_DIR/stop.sh"
            sleep 2
            "$INSTALL_DIR/start.sh"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        4)
            "$INSTALL_DIR/status.sh"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        5)
            "$INSTALL_DIR/logs.sh"
            ;;
        6)
            echo -e "${YELLOW}=== ข้อมูล Blockchain ===${NC}"
            
            # Try different RPC endpoints
            echo "กำลังตรวจสอบ RPC endpoints..."
            
            if curl -s --connect-timeout 3 http://localhost:19443/health 2>/dev/null; then
                echo -e "${GREEN}✅ RPC HTTP ทำงาน${NC}"
            elif curl -s --connect-timeout 3 https://localhost:19443/health --insecure 2>/dev/null; then
                echo -e "${GREEN}✅ RPC HTTPS ทำงาน${NC}"
            else
                echo -e "${RED}❌ RPC ไม่ตอบสนอง${NC}"
                echo "อาจต้องตั้งค่า RPC server แยกต่างหาก"
            fi
            
            echo ""
            echo -e "${YELLOW}=== ข้อมูลการเชื่อมต่อ ===${NC}"
            if lsof -i :19443 2>/dev/null | grep LISTEN >/dev/null; then
                echo -e "${GREEN}✅ Port 19443 เปิดอยู่${NC}"
                lsof -i :19443 2>/dev/null | grep LISTEN
            else
                echo -e "${RED}❌ Port 19443 ไม่เปิด${NC}"
            fi
            
            echo ""
            echo -e "${YELLOW}=== Process ที่ทำงาน ===${NC}"
            if [[ -f "$PID_FILE" ]]; then
                PID=$(cat "$PID_FILE")
                if kill -0 "$PID" 2>/dev/null; then
                    echo -e "${GREEN}✅ Node Process PID: $PID${NC}"
                    ps -p "$PID" -o pid,ppid,cmd 2>/dev/null || echo "ไม่สามารถดูข้อมูล process"
                else
                    echo -e "${RED}❌ Node Process ไม่ทำงาน${NC}"
                fi
            fi
            
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        7)
            echo -e "${YELLOW}กำลังสร้างที่อยู่ใหม่...${NC}"
            read -s -p "🔑 ใส่รหัสผ่าน wallet: " WALLET_PASS
            echo
            "$INSTALL_DIR/bin/bitquan-node" wallet-address --keystore "$INSTALL_DIR/config/pool-wallet.keystore" --password "$WALLET_PASS"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        8)
            echo -e "${YELLOW}กำลังตรวจสอบ balance...${NC}"
            read -s -p "🔑 ใส่รหัสผ่าน wallet: " WALLET_PASS
            echo
            ADDRESS=$("$INSTALL_DIR/bin/bitquan-node" wallet-address --keystore "$INSTALL_DIR/config/pool-wallet.keystore" --password "$WALLET_PASS" 2>/dev/null | grep "📍 Address:" | cut -d' ' -f3)
            if [[ -n "$ADDRESS" ]]; then
                "$INSTALL_DIR/bin/bitquan-node" balance --address "$ADDRESS" 2>/dev/null || echo "❌ ไม่สามารถตรวจสอบ balance"
            else
                echo "❌ ไม่สามารถดูที่อยู่ได้"
            fi
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        9)
            echo -e "${GREEN}ออกจากเมนู${NC}"
            exit 0
            ;;
        *)
            echo -e "${RED}❌ ตัวเลือกไม่ถูกต้อง${NC}"
            sleep 2
            ;;
    esac
}

while true; do
    show_menu
    handle_choice
done
EOF

    # Make all scripts executable
    chmod +x "$INSTALL_DIR"/*.sh
    echo "✅ สร้าง helper scripts เรียบร้อย"
}

# Function to start node
start_node() {
    echo -e "${YELLOW}🚀 กำลังเปิด BitQuan testnet node...${NC}"
    "$INSTALL_DIR/start.sh"
}

# Function to stop node
stop_node() {
    echo -e "${YELLOW}🛑 กำลังปิด BitQuan testnet node...${NC}"
    
    # Stop mining first
    if [[ -f "$MINING_PID_FILE" ]] && kill -0 $(cat "$MINING_PID_FILE") 2>/dev/null; then
        echo -e "${YELLOW}⛏️ กำลังหยุดการขุดก่อน...${NC}"
        stop_mining
    fi
    
    "$INSTALL_DIR/stop.sh"
}

# Function to show status
show_status() {
    echo -e "${YELLOW}📊 สถานะ BitQuan Testnet Node:${NC}"
    "$INSTALL_DIR/status.sh"
}

# Function to start mining
start_mining() {
    echo -e "${YELLOW}⛏️ กำลังเริ่มการขุด...${NC}"
    
    # Check if node is running
    if [[ ! -f "$PID_FILE" ]] || ! kill -0 $(cat "$PID_FILE") 2>/dev/null; then
        echo -e "${RED}❌ โหนดไม่ทำงาน กรุณาเปิดโหนดก่อน${NC}"
        return 1
    fi
    
    # Check if already mining
    if [[ -f "$MINING_PID_FILE" ]] && kill -0 $(cat "$MINING_PID_FILE") 2>/dev/null; then
        echo -e "${YELLOW}⚠️ กำลังขุดอยู่แล้ว${NC}"
        return 0
    fi
    
    # Get mining address
    read -s -p "🔑 ใส่รหัสผ่าน wallet: " WALLET_PASS
    echo
    MINING_ADDRESS=$("$INSTALL_DIR/bin/bitquan-node" wallet-address \
      --keystore "$CONFIG_DIR/pool-wallet.keystore" \
      --password "$WALLET_PASS" 2>/dev/null | grep "📍 Address:" | cut -d' ' -f3)
    
    if [[ -z "$MINING_ADDRESS" ]]; then
        echo -e "${RED}❌ ไม่สามารถดูที่อยู่ wallet ได้${NC}"
        return 1
    fi
    
    echo -e "${YELLOW}กำลังขุดไปที่: $MINING_ADDRESS${NC}"
    
    # Start mining in background
    cd "$INSTALL_DIR"
    nohup "$INSTALL_DIR/bin/bitquan-node" mine \
      --network testnet \
      --datadir "$DATA_DIR" \
      --payout-script-hex "$("$INSTALL_DIR/bin/bitquan-node" script-from-address --address "$MINING_ADDRESS" 2>/dev/null)" \
      --threads 1 \
      > "$INSTALL_DIR/logs/mining.log" 2>&1 &
    
    echo $! > "$MINING_PID_FILE"
    sleep 2
    
    if kill -0 $(cat "$MINING_PID_FILE") 2>/dev/null; then
        echo -e "${GREEN}✅ เริ่มการขุดแล้ว (PID: $(cat "$MINING_PID_FILE"))${NC}"
    else
        echo -e "${RED}❌ ไม่สามารถเริ่มการขุดได้${NC}"
        rm -f "$MINING_PID_FILE"
    fi
}

# Function to stop mining
stop_mining() {
    echo -e "${YELLOW}🛑 กำลังหยุดการขุด...${NC}"
    
    if [[ ! -f "$MINING_PID_FILE" ]]; then
        echo -e "${YELLOW}⚠️ ไม่มีการขุดอยู่${NC}"
        return 0
    fi
    
    MINING_PID=$(cat "$MINING_PID_FILE")
    if kill -0 "$MINING_PID" 2>/dev/null; then
        kill "$MINING_PID"
        
        # Wait for graceful shutdown
        for i in {1..10}; do
            if ! kill -0 "$MINING_PID" 2>/dev/null; then
                break
            fi
            sleep 1
        done
        
        # Force kill if still running
        if kill -0 "$MINING_PID" 2>/dev/null; then
            echo -e "${YELLOW}⚠️ บังคับหยุดการขุด...${NC}"
            kill -9 "$MINING_PID"
        fi
        
        echo -e "${GREEN}✅ หยุดการขุดเรียบร้อย${NC}"
    else
        echo -e "${YELLOW}⚠️ กระบวนการขุดไม่ทำงาน${NC}"
    fi
    
    rm -f "$MINING_PID_FILE"
}

# Function to start RPC server
start_rpc() {
    echo -e "${YELLOW}🌐 กำลังเปิด RPC server...${NC}"
    
    # Check if node is running
    if [[ ! -f "$PID_FILE" ]] || ! kill -0 $(cat "$PID_FILE") 2>/dev/null; then
        echo -e "${RED}❌ โหนดไม่ทำงาน กรุณาเปิดโหนดก่อน${NC}"
        return 1
    fi
    
    # Generate certificate if not exists
    if [[ ! -f "$CONFIG_DIR/cert.pem" ]]; then
        echo -e "${YELLOW}🔐 กำลังสร้าง RPC certificate...${NC}"
        "$INSTALL_DIR/bin/bitquan-node" generate-cert --output "$CONFIG_DIR"
    fi
    
    # Check if RPC is already running
    if curl -s --connect-timeout 2 http://localhost:19443/health >/dev/null 2>&1; then
        echo -e "${GREEN}✅ RPC server ทำงานอยู่แล้ว${NC}"
        return 0
    fi
    
    echo -e "${YELLOW}⚠️  RPC server อาจต้องเปิดแยกต่างหาก${NC}"
    echo "กรุณาตรวจสอบว่า BitQuan node รองรับ RPC หรือไม่"
    echo "หรือลองคำสั่ง: $INSTALL_DIR/bin/bitquan-node --help ดูว่ามี rpc command หรือไม่"
}

# Function to show logs
show_logs() {
    echo -e "${YELLOW}กำลังแสดง logs (กด Ctrl+C เพื่อออก):${NC}"
    "$INSTALL_DIR/logs.sh"
}

# Function to show menu
show_management_menu() {
    while true; do
        show_banner
        
        # Show different menu based on installation status
        if [[ -f "$INSTALL_DIR/bin/bitquan-node" ]] && [[ -f "$INSTALL_DIR/start.sh" ]]; then
        echo -e "${YELLOW}เลือกการทำงาน:${NC}"
        echo "  1) ติดตั้งใหม่ (ลบของเดิมทั้งหมด)"
        echo "  2) เปิดโหนด (Start Node)"
        echo "  3) ปิดโหนด (Stop Node)"
        echo "  4) รีสตาร์ทโหนด (Restart Node)"
            echo "  5) เริ่มขุด (Start Mining)"
            echo "  6) หยุดขุด (Stop Mining)"
            echo "  7) เปิด RPC server (Start RPC)"
            echo "  8) ดูสถานะละเอียด (View Status)"
            echo "  9) ดู logs (View Logs)"
            echo "  10) เมนูขั้นสูง (Advanced Menu)"
            echo "  11) ออก (Exit)"
            echo ""
            read -p "เลือกตัวเลือก (1-11): " choice
            
            case $choice in
                1)
                    clean_reinstall
                    ;;
                2)
                    start_node
                    read -p "กด Enter เพื่อดำเนินการต่อ..."
                    ;;
                3)
                    stop_node
                    read -p "กด Enter เพื่อดำเนินการต่อ..."
                    ;;
                4)
                    echo -e "${YELLOW}🔄 กำลังรีสตาร์ทโหนด...${NC}"
                    "$INSTALL_DIR/stop.sh"
                    sleep 2
                    "$INSTALL_DIR/start.sh"
                    read -p "กด Enter เพื่อดำเนินการต่อ..."
                    ;;
                5)
                    start_mining
                    read -p "กด Enter เพื่อดำเนินการต่อ..."
                    ;;
                6)
                    stop_mining
                    read -p "กด Enter เพื่อดำเนินการต่อ..."
                    ;;
                7)
                    start_rpc
                    read -p "กด Enter เพื่อดำเนินการต่อ..."
                    ;;
                8)
                    show_status
                    read -p "กด Enter เพื่อดำเนินการต่อ..."
                    ;;
                9)
                    show_logs
                    ;;
                10)
                    "$INSTALL_DIR/menu.sh"
                    ;;
                11)
                    echo -e "${GREEN}ลาก่อน! 👋${NC}"
                    exit 0
                    ;;
                *)
                    echo -e "${RED}❌ ตัวเลือกไม่ถูกต้อง กรุณาเลือก 1-11${NC}"
                    sleep 2
                    ;;
            esac
        else
            echo -e "${YELLOW}เลือกการทำงาน:${NC}"
            echo "  1) ติดตั้งโหนดใหม่ (Full Setup)"
            echo "  2) ออก (Exit)"
            echo ""
            read -p "เลือกตัวเลือก (1-2): " choice
            
            case $choice in
                1)
                    full_setup
                    ;;
                2)
                    echo -e "${GREEN}ลาก่อน! 👋${NC}"
                    exit 0
                    ;;
                *)
                    echo -e "${RED}❌ ตัวเลือกไม่ถูกต้อง กรุณาเลือก 1-2${NC}"
                    sleep 2
                    ;;
            esac
        fi
    done
}

# Function for clean reinstall
clean_reinstall() {
    echo -e "${RED}⚠️  คำเตือน: การดำเนินการนี้จะลบข้อมูลทั้งหมด!${NC}"
    echo -e "${YELLOW}ข้อมูลที่จะถูกลบ:${NC}"
    echo "  • Wallet และ private keys"
    echo "  • ข้อมูล blockchain"
    echo "  • Configuration files"
    echo "  • Logs ทั้งหมด"
    echo ""
    
    read -p "❌ ยืนยันการลบข้อมูลทั้งหมด? (พิมพ์ 'DELETE' เพื่อยืนยัน): " confirm
    if [[ "$confirm" != "DELETE" ]]; then
        echo "❌ ยกเลิกการดำเนินการ"
        return 0
    fi
    
    echo -e "${YELLOW}🗑️ กำลังลบข้อมูลเก่า...${NC}"
    
    # Stop node if running
    if [[ -f "$PID_FILE" ]] && kill -0 $(cat "$PID_FILE") 2>/dev/null; then
        echo "กำลังปิดโหนด..."
        kill $(cat "$PID_FILE") 2>/dev/null || true
        sleep 2
    fi
    
    # Stop mining if running
    if [[ -f "$MINING_PID_FILE" ]] && kill -0 $(cat "$MINING_PID_FILE") 2>/dev/null; then
        echo "กำลังหยุดการขุด..."
        kill $(cat "$MINING_PID_FILE") 2>/dev/null || true
        sleep 2
    fi
    
    # Remove installation directory
    if [[ -d "$INSTALL_DIR" ]]; then
        rm -rf "$INSTALL_DIR"
        echo "✅ ลบ $INSTALL_DIR เรียบร้อย"
    fi
    
    echo -e "${GREEN}✅ ลบข้อมูลเก่าเรียบร้อยแล้ว${NC}"
    echo ""
    
    # Start fresh installation
    full_setup
}

# Function for full setup
full_setup() {
    echo -e "${YELLOW}🚀 เริ่มการติดตั้ง BitQuan Testnet Node แบบอัตโนมัติ...${NC}"
    echo ""
    
    # Ask for confirmation
    read -p "ต้องการติดตั้ง BitQuan Testnet Node หรือไม่? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "ยกเลิกการติดตั้ง"
        return 0
    fi
    
    echo ""
    echo -e "${GREEN}เริ่มกระบวนการติดตั้ง...${NC}"
    echo ""
    
    # Run all setup steps
    install_dependencies
    setup_directories
    get_binary
    generate_jwt_secret
    create_wallet
    create_config
    create_helper_scripts
    
    # Start the node
    start_node
    
    # Print summary
    echo ""
    echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║          การติดตั้ง BitQuan Testnet เสร็จสมบูรณ์! 🎉          ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}📊 ข้อมูลโหนด:${NC}"
    echo "  ไดเรกทอรีติดตั้ง: $INSTALL_DIR"
    echo "  ไดเรกทอรีข้อมูล: $DATA_DIR"
    echo "  Mining Address: $MINING_ADDRESS"
    echo ""
    echo -e "${YELLOW}🌐 พอร์ตที่เปิด:${NC}"
    echo "  P2P Port: 19444"
    echo "  RPC Port: 19443"
    echo ""
    echo -e "${YELLOW}🛠️ คำสั่งที่ใช้:${NC}"
    echo "  เข้าถึงเมนู: $INSTALL_DIR/menu.sh"
    echo "  เปิดโหนด: $INSTALL_DIR/start.sh"
    echo "  ปิดโหนด: $INSTALL_DIR/stop.sh"
    echo "  ดูสถานะ: $INSTALL_DIR/status.sh"
    echo "  ดู logs: $INSTALL_DIR/logs.sh"
    echo ""
    echo -e "${YELLOW}⚠️  ข้อควรระวัง:${NC}"
    echo "  • สำรองข้อมูล wallet: $CONFIG_DIR/pool-wallet.keystore"
    echo "  • JWT secret: $CONFIG_DIR/jwt.secret"
    echo "  • ตรวจสอบ logs เป็นประจำ"
    echo ""
    echo -e "${GREEN}ทดสอบ BitQuan ให้สนุก! 🚀${NC}"
    
    read -p "กด Enter เพื่อดำเนินการต่อ..."
}

# Main function
main() {
    check_macos
    show_management_menu
}

# Run main function
main "$@"
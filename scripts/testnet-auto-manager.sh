#!/usr/bin/env bash
# BitQuan Testnet Auto Management Script
# One-command setup and management for testnet nodes

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Configuration
BITQUAN_USER="bitquan"
INSTALL_DIR="/opt/bitquan"
DATA_DIR="/opt/bitquan/data/testnet"
CONFIG_DIR="/opt/bitquan/config"
SERVICE_NAME="bitquan-testnet"

# Function to display banner
show_banner() {
    clear
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║         BitQuan Testnet Auto Manager v2.0.0                   ║"
    echo "║         จัดการโหนด Testnet อัตโนมัติทั้งหมดในคำสั่งเดียว            ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

# Function to check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        echo -e "${RED}❌ ต้องรันสคริปนี้ด้วยสิทธิ์ root (ใช้ sudo)${NC}"
        exit 1
    fi
}

# Function to detect OS
detect_os() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        OS=$ID
    else
        echo -e "${RED}❌ ไม่สามารถตรวจสอบระบบปฏิบัติการได้${NC}"
        exit 1
    fi
}

# Function to install dependencies
install_dependencies() {
    echo -e "${YELLOW}📦 กำลังติดตั้ง dependencies...${NC}"
    
    if [[ "$OS" == "ubuntu" ]] || [[ "$OS" == "debian" ]]; then
        apt update -qq
        apt install -y curl wget git build-essential pkg-config libssl-dev python3 python3-pip jq docker.io docker-compose
        systemctl enable docker
        systemctl start docker
    elif [[ "$OS" == "centos" ]] || [[ "$OS" == "rhel" ]] || [[ "$OS" == "fedora" ]]; then
        if [[ "$OS" == "fedora" ]]; then
            dnf install -y curl wget git gcc gcc-c++ make openssl-devel python3 python3-pip jq docker docker-compose
        else
            yum install -y curl wget git gcc gcc-c++ make openssl-devel python3 python3-pip jq docker docker-compose
        fi
        systemctl enable docker
        systemctl start docker
    fi
    
    echo -e "${GREEN}✅ ติดตั้ง dependencies เรียบร้อย${NC}"
}

# Function to create user and directories
setup_user_directories() {
    echo -e "${YELLOW}👤 กำลังสร้าง user และ directories...${NC}"
    
    # Create user
    if ! id "$BITQUAN_USER" &>/dev/null; then
        useradd -m -s /bin/bash "$BITQUAN_USER"
        usermod -aG docker "$BITQUAN_USER"
        echo "✅ สร้าง user $BITQUAN_USER"
    else
        echo "✅ user $BITQUAN_USER มีอยู่แล้ว"
    fi
    
    # Create directories
    mkdir -p "$INSTALL_DIR"/{bin,data,logs,backups,config}
    mkdir -p "$DATA_DIR"
    mkdir -p "$CONFIG_DIR"
    chown -R "$BITQUAN_USER":"$BITQUAN_USER" "$INSTALL_DIR"
    echo "✅ สร้าง directories เรียบร้อย"
}

# Function to get BitQuan binary
get_binary() {
    echo -e "${YELLOW}⬇️ กำลังดาวน์โหลด BitQuan binary...${NC}"
    
    # Try to download from releases first
    BIN_URL="https://github.com/AlphaB135/BitQuan/releases/download/v1.0.0/bitquan-linux-x86_64"
    if curl -fsSL "$BIN_URL" -o "$INSTALL_DIR/bin/bitquan-node" 2>/dev/null; then
        echo "✅ ดาวน์โหลดจาก GitHub releases"
    else
        echo "⚠️ ไม่พบ release กำลัง build จาก source..."
        
        # Install Rust
        if ! command -v cargo &> /dev/null; then
            echo "📦 กำลังติดตั้ง Rust..."
            sudo -u "$BITQUAN_USER" bash -c 'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y'
            sudo -u "$BITQUAN_USER" bash -c 'source $HOME/.cargo/env'
        fi
        
        # Clone and build
        cd /tmp
        if [[ -d BitQuan ]]; then
            rm -rf BitQuan
        fi
        git clone https://github.com/AlphaB135/BitQuan.git BitQuan
        cd BitQuan
        git checkout v1.0.0 || git checkout main
        sudo -u "$BITQUAN_USER" bash -c 'source $HOME/.cargo/env && cargo build --release --bin bitquan-node'
        cp target/release/bitquan-node "$INSTALL_DIR/bin/"
        cd /tmp
        rm -rf BitQuan
        echo "✅ Build จาก source เรียบร้อย"
    fi
    
    chmod +x "$INSTALL_DIR/bin/bitquan-node"
    chown "$BITQUAN_USER":"$BITQUAN_USER" "$INSTALL_DIR/bin/bitquan-node"
}

# Function to generate JWT secret
generate_jwt_secret() {
    echo -e "${YELLOW}🔐 กำลังสร้าง JWT secret...${NC}"
    JWT_SECRET=$(openssl rand -hex 32)
    echo "$JWT_SECRET" > "$CONFIG_DIR/jwt.secret"
    chmod 600 "$CONFIG_DIR/jwt.secret"
    chown "$BITQUAN_USER":"$BITQUAN_USER" "$CONFIG_DIR/jwt.secret"
    echo "✅ สร้าง JWT secret เรียบร้อย"
}

# Function to create wallet
create_wallet() {
    echo -e "${YELLOW}💰 กำลังสร้าง testnet wallet...${NC}"
    
    # Ask for wallet password
    while true; do
        read -s -p "🔑 ใส่รหัสผ่านสำหรับ wallet: " WALLET_PASSWORD
        echo
        read -s -p "🔑 ยืนยันรหัสผ่าน: " WALLET_PASSWORD_CONFIRM
        echo
        if [[ "$WALLET_PASSWORD" == "$WALLET_PASSWORD_CONFIRM" ]]; then
            break
        else
            echo -e "${RED}❌ รหัสผ่านไม่ตรงกัน กรุณาลองใหม่${NC}"
        fi
    done
    
    # Create wallet
    sudo -u "$BITQUAN_USER" "$INSTALL_DIR/bin/bitquan-node" wallet create \
      --network testnet \
      --output "$CONFIG_DIR/pool-wallet.keystore" \
      --password "$WALLET_PASSWORD" || true
    
    # Get mining address
    MINING_ADDRESS=$(sudo -u "$BITQUAN_USER" "$INSTALL_DIR/bin/bitquan-node" wallet address \
      --keystore "$CONFIG_DIR/pool-wallet.keystore" 2>/dev/null | tail -1 || echo "tBQ1_CHANGE_ME")
    echo "✅ Mining address: $MINING_ADDRESS"
}

# Function to create configuration
create_config() {
    echo -e "${YELLOW}⚙️ กำลังสร้าง configuration...${NC}"
    
    cat > "$CONFIG_DIR/testnet.toml" << EOF
# BitQuan Testnet Configuration

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
bind = "0.0.0.0:19443"
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

    chown "$BITQUAN_USER":"$BITQUAN_USER" "$CONFIG_DIR/testnet.toml"
    echo "✅ สร้าง configuration เรียบร้อย"
}

# Function to create systemd service
create_service() {
    echo -e "${YELLOW}🔧 กำลังสร้าง systemd service...${NC}"
    
    cat > /etc/systemd/system/$SERVICE_NAME.service << EOF
[Unit]
Description=BitQuan Testnet Node
After=network.target docker.service

[Service]
Type=simple
User=$BITQUAN_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/bin/bitquan-node \\
  --network testnet \\
  --data-dir $DATA_DIR \\
  --config $CONFIG_DIR/testnet.toml

Restart=always
RestartSec=10
StandardOutput=append:$INSTALL_DIR/logs/testnet.log
StandardError=append:$INSTALL_DIR/logs/testnet-error.log

NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$INSTALL_DIR/data
ReadWritePaths=$INSTALL_DIR/logs

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    echo "✅ สร้าง systemd service เรียบร้อย"
}

# Function to configure firewall
configure_firewall() {
    echo -e "${YELLOW}🔥 กำลังตั้งค่า firewall...${NC}"
    
    if command -v ufw &> /dev/null; then
        ufw allow 19444/tcp comment 'BitQuan P2P'
        ufw allow 19443/tcp comment 'BitQuan RPC'
        echo "✅ เพิ่ม UFW rules"
    elif command -v firewall-cmd &> /dev/null; then
        firewall-cmd --permanent --add-port=19444/tcp
        firewall-cmd --permanent --add-port=19443/tcp
        firewall-cmd --reload
        echo "✅ เพิ่ม Firewalld rules"
    else
        echo "⚠️ ไม่พบ firewall กรุณาตั้งค่าด้วยตนเอง"
    fi
}

# Function to create helper scripts
create_helper_scripts() {
    echo -e "${YELLOW}📝 กำลังสร้าง helper scripts...${NC}"
    
    # Menu script
    cat > "$INSTALL_DIR/menu.sh" << 'EOF'
#!/bin/bash
# BitQuan Testnet Management Menu

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

show_menu() {
    clear
    echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║           BitQuan Testnet Management Menu                   ║${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "${YELLOW}📊 สถานะโหนด:${NC}"
    if systemctl is-active --quiet bitquan-testnet; then
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
    echo "  9) อัปเดตโหนด (Update Node)"
    echo "  10) ออกจากเมนู (Exit)"
    echo ""
    read -p "เลือกตัวเลือก (1-10): " choice
}

handle_choice() {
    case $choice in
        1)
            echo -e "${YELLOW}กำลังเปิดโหนด...${NC}"
            sudo systemctl start bitquan-testnet
            sleep 2
            echo -e "${GREEN}✅ เปิดโหนดเรียบร้อย${NC}"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        2)
            echo -e "${YELLOW}กำลังปิดโหนด...${NC}"
            sudo systemctl stop bitquan-testnet
            echo -e "${GREEN}✅ ปิดโหนดเรียบร้อย${NC}"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        3)
            echo -e "${YELLOW}กำลังรีสตาร์ทโหนด...${NC}"
            sudo systemctl restart bitquan-testnet
            sleep 2
            echo -e "${GREEN}✅ รีสตาร์ทโหนดเรียบร้อย${NC}"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        4)
            echo -e "${YELLOW}=== สถานะโหนด ===${NC}"
            sudo systemctl status bitquan-testnet --no-pager
            echo ""
            echo -e "${YELLOW}=== พอร์ตที่เปิด ===${NC}"
            netstat -tlnp | grep -E ':(19443|19444)' || echo "ไม่มีพอร์ตเปิด"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        5)
            echo -e "${YELLOW}กำลังแสดง logs (กด q เพื่อออก):${NC}"
            sudo journalctl -u bitquan-testnet -f
            ;;
        6)
            echo -e "${YELLOW}=== ข้อมูล Blockchain ===${NC}"
            curl -s http://localhost:19443/health 2>/dev/null || echo "❌ RPC ไม่ตอบสนอง"
            echo ""
            echo -e "${YELLOW}=== ข้อมูลการเชื่อมต่อ ===${NC}"
            curl -s http://localhost:19443/peers 2>/dev/null || echo "❌ ไม่สามารถดูข้อมูล peers"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        7)
            echo -e "${YELLOW}กำลังสร้างที่อยู่ใหม่...${NC}"
            /opt/bitquan/bin/bitquan-node wallet address --keystore /opt/bitquan/config/pool-wallet.keystore
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        8)
            echo -e "${YELLOW}กำลังตรวจสอบ balance...${NC}"
            /opt/bitquan/bin/bitquan-node wallet balance --keystore /opt/bitquan/config/pool-wallet.keystore 2>/dev/null || echo "❌ ไม่สามารถตรวจสอบ balance"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        9)
            echo -e "${YELLOW}กำลังอัปเดตโหนด...${NC}"
            echo "ฟีเจอร์นี้จะมาในเวอร์ชันถัดไป"
            read -p "กด Enter เพื่อดำเนินการต่อ..."
            ;;
        10)
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

    chmod +x "$INSTALL_DIR/menu.sh"
    chown "$BITQUAN_USER":"$BITQUAN_USER" "$INSTALL_DIR/menu.sh"
    
    echo "✅ สร้าง helper scripts เรียบร้อย"
}

# Function to start node
start_node() {
    echo -e "${YELLOW}🚀 กำลังเปิด BitQuan testnet node...${NC}"
    systemctl enable $SERVICE_NAME
    systemctl start $SERVICE_NAME
    sleep 3
    
    if systemctl is-active --quiet $SERVICE_NAME; then
        echo -e "${GREEN}✅ BitQuan testnet node กำลังทำงาน!${NC}"
    else
        echo -e "${RED}❌ ไม่สามารถเปิดโหนดได้${NC}"
        echo "ตรวจสอบ logs: sudo journalctl -u $SERVICE_NAME -n 50"
        return 1
    fi
}

# Function to stop node
stop_node() {
    echo -e "${YELLOW}🛑 กำลังปิด BitQuan testnet node...${NC}"
    systemctl stop $SERVICE_NAME
    echo -e "${GREEN}✅ ปิดโหนดเรียบร้อย${NC}"
}

# Function to show status
show_status() {
    echo -e "${YELLOW}📊 สถานะ BitQuan Testnet Node:${NC}"
    echo ""
    
    if systemctl is-active --quiet $SERVICE_NAME; then
        echo -e "Status: ${GREEN}● กำลังทำงาน${NC}"
    else
        echo -e "Status: ${RED}● หยุดทำงาน${NC}"
    fi
    
    echo ""
    echo -e "${YELLOW}=== Service Status ===${NC}"
    systemctl status $SERVICE_NAME --no-pager -l
    
    echo ""
    echo -e "${YELLOW}=== Network Ports ===${NC}"
    netstat -tlnp | grep -E ':(19443|19444)' || echo "ไม่มีพอร์ตเปิด"
    
    echo ""
    echo -e "${YELLOW}=== Recent Logs ===${NC}"
    sudo journalctl -u $SERVICE_NAME -n 10 --no-pager
}

# Function to show logs
show_logs() {
    echo -e "${YELLOW}กำลังแสดง logs (กด Ctrl+C เพื่อออก):${NC}"
    sudo journalctl -u $SERVICE_NAME -f
}

# Function to show menu
show_management_menu() {
    while true; do
        show_banner
        echo -e "${YELLOW}📊 สถานะปัจจุบัน:${NC}"
        if systemctl is-active --quiet $SERVICE_NAME 2>/dev/null; then
            echo -e "  โหนด: ${GREEN}● กำลังทำงาน${NC}"
        else
            echo -e "  โหนด: ${RED}● หยุดทำงาน${NC}"
        fi
        echo ""
        
        echo -e "${YELLOW}🛠️  เลือกการทำงาน:${NC}"
        echo "  1) 🚀 ติดตั้งโหนดใหม่ (Full Setup)"
        echo "  2) ▶️  เปิดโหนด (Start Node)"
        echo "  3) ⏸️  ปิดโหนด (Stop Node)"
        echo "  4) 🔄 รีสตาร์ทโหนด (Restart Node)"
        echo "  5) 📊 ดูสถานะ (View Status)"
        echo "  6) 📋 ดู logs (View Logs)"
        echo "  7) 🎛️  เข้าถึงเมนูจัดการขั้นสูง (Advanced Menu)"
        echo "  8) ❌ ออก (Exit)"
        echo ""
        
        read -p "เลือกตัวเลือก (1-8): " choice
        
        case $choice in
            1)
                full_setup
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
                systemctl restart $SERVICE_NAME
                sleep 2
                echo -e "${GREEN}✅ รีสตาร์ทเรียบร้อย${NC}"
                read -p "กด Enter เพื่อดำเนินการต่อ..."
                ;;
            5)
                show_status
                read -p "กด Enter เพื่อดำเนินการต่อ..."
                ;;
            6)
                show_logs
                ;;
            7)
                sudo -u "$BITQUAN_USER" "$INSTALL_DIR/menu.sh"
                ;;
            8)
                echo -e "${GREEN}ลาก่อน! 👋${NC}"
                exit 0
                ;;
            *)
                echo -e "${RED}❌ ตัวเลือกไม่ถูกต้อง กรุณาเลือก 1-8${NC}"
                sleep 2
                ;;
        esac
    done
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
    detect_os
    install_dependencies
    setup_user_directories
    get_binary
    generate_jwt_secret
    create_wallet
    create_config
    create_service
    configure_firewall
    create_helper_scripts
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
    echo "  เข้าถึงเมนู: sudo -u $BITQUAN_USER $INSTALL_DIR/menu.sh"
    echo "  เปิดโหนด: sudo systemctl start $SERVICE_NAME"
    echo "  ปิดโหนด: sudo systemctl stop $SERVICE_NAME"
    echo "  ดูสถานะ: sudo systemctl status $SERVICE_NAME"
    echo "  ดู logs: sudo journalctl -u $SERVICE_NAME -f"
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
    check_root
    
    # Check if already installed
    if [[ -f "$INSTALL_DIR/bin/bitquan-node" ]] && [[ -f "/etc/systemd/system/$SERVICE_NAME.service" ]]; then
        show_management_menu
    else
        show_banner
        echo -e "${YELLOW}ยินดีต้อนรับสู่ BitQuan Testnet Auto Manager!${NC}"
        echo ""
        echo -e "${YELLOW}สคริปนี้จะติดตั้งและจัดการ BitQuan Testnet Node อัตโนมัติ${NC}"
        echo ""
        echo -e "${YELLOW}ฟีเจอร์:${NC}"
        echo "  ✅ ติดตั้งทุกอย่างอัตโนมัติ"
        echo "  ✅ สร้าง wallet พร้อมรหัสผ่าน"
        echo "  ✅ ตั้งค่า firewall"
        echo "  ✅ สร้าง systemd service"
        echo "  ✅ เมนูจัดการโหนด"
        echo ""
        
        full_setup
        show_management_menu
    fi
}

# Run main function
main "$@"
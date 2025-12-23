#!/bin/bash

# BitQuan Backup and Recovery Script
# ระบบ backup แบบครบวงจรสำหรับ blockchain node

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/opt/backups/bitquan}"
CONFIG_DIR="${CONFIG_DIR:-/etc/bitquan}"
DATA_DIR="${DATA_DIR:-/var/lib/bitquan}"
LOG_FILE="${LOG_FILE:-/var/log/bitquan/backup.log}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
BACKUP_TYPE="${BACKUP_TYPE:-full}" # full, incremental, config-only
COMPRESS="${COMPRESS:-true}"
ENCRYPT="${ENCRYPT:-true}"
GPG_RECIPIENT="${GPG_RECIPIENT:-backup@bitquan.org}"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Global variables
BACKUP_DATE=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="bitquan_backup_${BACKUP_DATE}"
TEMP_DIR="/tmp/bitquan_backup_${BACKUP_DATE}"
BACKUP_SUCCESS=false

# Initialize logging
setup_logging() {
    mkdir -p "$(dirname "$LOG_FILE")"
    mkdir -p "$BACKUP_DIR"
    mkdir -p "$TEMP_DIR"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] Backup process started" >> "$LOG_FILE"
}

log_message() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    # Log to file
    echo "$(date '+%Y-%m-%d %H:%M:%S') [$level] $message" >> "$LOG_FILE"

    # Console output with colors
    case "$level" in
        "ERROR")
            echo -e "${RED}[ERROR]${NC} $message"
            ;;
        "WARNING")
            echo -e "${YELLOW}[WARNING]${NC} $message"
            ;;
        "INFO")
            echo -e "${GREEN}[INFO]${NC} $message"
            ;;
        "DEBUG")
            echo -e "${BLUE}[DEBUG]${NC} $message"
            ;;
    esac
}

# Check prerequisites
check_prerequisites() {
    log_message "INFO" "Checking backup prerequisites..."

    local required_commands=("tar" "gzip" "find" "cp" "rsync")
    if [[ "$ENCRYPT" == "true" ]]; then
        required_commands+=("gpg")
    fi

    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            log_message "ERROR" "Required command not found: $cmd"
            exit 1
        fi
    done

    # Check directories
    if [[ ! -d "$CONFIG_DIR" ]]; then
        log_message "ERROR" "Configuration directory not found: $CONFIG_DIR"
        exit 1
    fi

    if [[ ! -d "$DATA_DIR" ]]; then
        log_message "WARNING" "Data directory not found: $DATA_DIR"
    fi

    # Check disk space
    local available_space=$(df -k "$BACKUP_DIR" | tail -1 | awk '{print $4}')
    local required_space=1048576 # 1GB in KB

    if [[ "$available_space" -lt "$required_space" ]]; then
        log_message "ERROR" "Insufficient disk space. Available: ${available_space}KB, Required: ${required_space}KB"
        exit 1
    fi

    log_message "INFO" "Prerequisites check completed successfully"
}

# Stop BitQuan service safely
stop_bitquan_service() {
    log_message "INFO" "Stopping BitQuan service for backup..."

    if systemctl is-active --quiet bitquan 2>/dev/null; then
        log_message "INFO" "BitQuan service is running, stopping gracefully..."
        systemctl stop bitquan || {
            log_message "WARNING" "Failed to stop BitQuan service with systemctl, trying manual stop..."
            pkill -f bitquan || log_message "WARNING" "Could not find BitQuan process to stop"
        }

        # Wait for service to stop
        local timeout=30
        local elapsed=0
        while systemctl is-active --quiet bitquan 2>/dev/null && [[ $elapsed -lt $timeout ]]; do
            sleep 1
            ((elapsed++))
        done

        if systemctl is-active --quiet bitquan 2>/dev/null; then
            log_message "ERROR" "Failed to stop BitQuan service within ${timeout} seconds"
            exit 1
        fi
    else
        log_message "INFO" "BitQuan service is not running"
    fi

    log_message "INFO" "BitQuan service stopped successfully"
}

# Start BitQuan service
start_bitquan_service() {
    log_message "INFO" "Starting BitQuan service after backup..."

    if command -v systemctl &> /dev/null; then
        systemctl start bitquan || {
            log_message "ERROR" "Failed to start BitQuan service"
            return 1
        }

        # Wait for service to start
        local timeout=30
        local elapsed=0
        while ! systemctl is-active --quiet bitquan && [[ $elapsed -lt $timeout ]]; do
            sleep 1
            ((elapsed++))
        done

        if systemctl is-active --quiet bitquan; then
            log_message "INFO" "BitQuan service started successfully"
            return 0
        else
            log_message "ERROR" "BitQuan service failed to start within ${timeout} seconds"
            return 1
        fi
    else
        log_message "WARNING" "systemctl not available, manual service start required"
        return 1
    fi
}

# Create configuration backup
backup_configuration() {
    log_message "INFO" "Creating configuration backup..."

    local config_backup="$TEMP_DIR/config"
    mkdir -p "$config_backup"

    # Backup configuration files
    if [[ -d "$CONFIG_DIR" ]]; then
        cp -r "$CONFIG_DIR"/* "$config_backup/" 2>/dev/null || {
            log_message "WARNING" "Some configuration files could not be copied"
        }
    fi

    # Backup systemd service files
    if [[ -d "/etc/systemd/system" ]]; then
        find /etc/systemd/system -name "*bitquan*" -exec cp {} "$config_backup/" \; 2>/dev/null || true
    fi

    # Backup environment files
    find /opt/bitquan -name ".env*" -exec cp {} "$config_backup/" \; 2>/dev/null || true

    # Backup logs configuration
    if [[ -d "/etc/logrotate.d" ]]; then
        find /etc/logrotate.d -name "*bitquan*" -exec cp {} "$config_backup/" \; 2>/dev/null || true
    fi

    log_message "INFO" "Configuration backup completed"
}

# Create blockchain data backup
backup_blockchain_data() {
    log_message "INFO" "Creating blockchain data backup..."

    if [[ ! -d "$DATA_DIR" ]]; then
        log_message "WARNING" "Data directory not found, skipping blockchain data backup"
        return 0
    fi

    local data_backup="$TEMP_DIR/data"
    mkdir -p "$data_backup"

    # Create blockchain data backup
    if [[ "$BACKUP_TYPE" == "incremental" ]]; then
        log_message "INFO" "Creating incremental backup..."
        rsync -a --link-dest="$BACKUP_DIR/latest/data" "$DATA_DIR/" "$data_backup/" || {
            log_message "ERROR" "Incremental backup failed"
            return 1
        }
    else
        log_message "INFO" "Creating full backup..."
        cp -r "$DATA_DIR" "$data_backup/" || {
            log_message "ERROR" "Full data backup failed"
            return 1
        }
    fi

    # Create data integrity checksum
    log_message "INFO" "Creating data integrity checksums..."
    find "$data_backup" -type f -exec sha256sum {} \; > "$data_backup/sha256sums.txt"

    log_message "INFO" "Blockchain data backup completed"
}

# Create system state backup
backup_system_state() {
    log_message "INFO" "Creating system state backup..."

    local state_backup="$TEMP_DIR/state"
    mkdir -p "$state_backup"

    # Backup current system state
    {
        echo "=== System Information ==="
        uname -a
        echo ""
        echo "=== Disk Usage ==="
        df -h
        echo ""
        echo "=== Memory Usage ==="
        free -h
        echo ""
        echo "=== Network Configuration ==="
        ip addr show
        echo ""
        echo "=== Running Processes ==="
        ps aux | grep -E "(bitquan|bitcoin|crypto)" || echo "No crypto processes found"
        echo ""
        echo "=== Firewall Rules ==="
        iptables -L 2>/dev/null || echo "iptables not available"
        echo ""
        echo "=== System Uptime ==="
        uptime
    } > "$state_backup/system_state.txt"

    # Backup recent logs
    if [[ -f "$LOG_FILE" ]]; then
        tail -1000 "$LOG_FILE" > "$state_backup/recent_logs.txt"
    fi

    log_message "INFO" "System state backup completed"
}

# Create application metadata
backup_metadata() {
    log_message "INFO" "Creating backup metadata..."

    local metadata_file="$TEMP_DIR/metadata.json"

    cat > "$metadata_file" << EOF
{
    "backup_type": "$BACKUP_TYPE",
    "backup_date": "$(date -Iseconds)",
    "backup_name": "$BACKUP_NAME",
    "hostname": "$(hostname)",
    "bitquan_version": "$(bitquan --version 2>/dev/null || echo 'unknown')",
    "git_commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
    "git_branch": "$(git branch --show-current 2>/dev/null || echo 'unknown')",
    "system_info": {
        "os": "$(uname -s)",
        "kernel": "$(uname -r)",
        "architecture": "$(uname -m)"
    },
    "backup_config": {
        "compress": $COMPRESS,
        "encrypt": $ENCRYPT,
        "retention_days": $RETENTION_DAYS
    },
    "directories": {
        "config_dir": "$CONFIG_DIR",
        "data_dir": "$DATA_DIR",
        "backup_dir": "$BACKUP_DIR"
    },
    "backup_size": "$(du -sb "$TEMP_DIR" | cut -f1)"
}
EOF

    log_message "INFO" "Backup metadata created"
}

# Compress backup
compress_backup() {
    if [[ "$COMPRESS" != "true" ]]; then
        log_message "INFO" "Compression disabled, skipping..."
        return 0
    fi

    log_message "INFO" "Compressing backup..."

    local compressed_file="$BACKUP_DIR/${BACKUP_NAME}.tar.gz"

    tar -czf "$compressed_file" -C "$TEMP_DIR" . || {
        log_message "ERROR" "Backup compression failed"
        return 1
    }

    # Verify compressed file
    if [[ ! -f "$compressed_file" ]]; then
        log_message "ERROR" "Compressed backup file not created"
        return 1
    fi

    local backup_size=$(stat -f%z "$compressed_file" 2>/dev/null || stat -c%s "$compressed_file")
    log_message "INFO" "Backup compressed successfully. Size: $((backup_size / 1024 / 1024))MB"

    echo "$compressed_file"
}

# Encrypt backup
encrypt_backup() {
    if [[ "$ENCRYPT" != "true" ]]; then
        log_message "INFO" "Encryption disabled, skipping..."
        return 0
    fi

    local backup_file="$1"
    local encrypted_file="${backup_file}.gpg"

    log_message "INFO" "Encrypting backup..."

    gpg --batch --yes --encrypt --recipient "$GPG_RECIPIENT" \
        --output "$encrypted_file" "$backup_file" || {
        log_message "ERROR" "Backup encryption failed"
        return 1
    }

    # Remove unencrypted file
    rm "$backup_file" || log_message "WARNING" "Could not remove unencrypted backup file"

    # Verify encrypted file
    if [[ ! -f "$encrypted_file" ]]; then
        log_message "ERROR" "Encrypted backup file not created"
        return 1
    fi

    local encrypted_size=$(stat -f%z "$encrypted_file" 2>/dev/null || stat -c%s "$encrypted_file")
    log_message "INFO" "Backup encrypted successfully. Size: $((encrypted_size / 1024 / 1024))MB"

    echo "$encrypted_file"
}

# Cleanup old backups
cleanup_old_backups() {
    log_message "INFO" "Cleaning up old backups (retention: $RETENTION_DAYS days)..."

    find "$BACKUP_DIR" -name "bitquan_backup_*.tar.gz*" -type f \
        -mtime +$RETENTION_DAYS -delete || {
        log_message "WARNING" "Some old backup files could not be deleted"
    }

    # Also cleanup directories
    find "$BACKUP_DIR" -name "bitquan_backup_*" -type d \
        -mtime +$RETENTION_DAYS -exec rm -rf {} + 2>/dev/null || true

    log_message "INFO" "Old backup cleanup completed"
}

# Create latest symlink
create_latest_symlink() {
    local backup_file="$1"
    local latest_link="$BACKUP_DIR/latest"

    log_message "INFO" "Creating latest backup symlink..."

    # Remove existing symlink
    rm -f "$latest_link"

    # Create new symlink
    ln -s "$backup_file" "$latest_link" || {
        log_message "WARNING" "Could not create latest backup symlink"
    }

    log_message "INFO" "Latest backup symlink created"
}

# Verify backup integrity
verify_backup() {
    local backup_file="$1"

    log_message "INFO" "Verifying backup integrity..."

    if [[ "$ENCRYPT" == "true" ]]; then
        # Decrypt to temp file for verification
        local temp_decrypted="/tmp/backup_verify_$$.tar.gz"
        gpg --batch --yes --decrypt --output "$temp_decrypted" "$backup_file" || {
            log_message "ERROR" "Backup decryption failed during verification"
            return 1
        }

        # Test archive integrity
        tar -tzf "$temp_decrypted" > /dev/null || {
            log_message "ERROR" "Compressed backup integrity check failed"
            rm -f "$temp_decrypted"
            return 1
        }

        rm -f "$temp_decrypted"
    else
        # Test archive integrity directly
        tar -tzf "$backup_file" > /dev/null || {
            log_message "ERROR" "Compressed backup integrity check failed"
            return 1
        }
    fi

    # Verify metadata exists
    if [[ "$ENCRYPT" == "true" ]]; then
        local metadata_content=$(gpg --batch --yes --decrypt "$backup_file" 2>/dev/null | tar -xO metadata.json 2>/dev/null || echo "")
    else
        local metadata_content=$(tar -xOf "$backup_file" metadata.json 2>/dev/null || echo "")
    fi

    if [[ -z "$metadata_content" ]]; then
        log_message "ERROR" "Backup metadata not found or corrupted"
        return 1
    fi

    log_message "INFO" "Backup integrity verification completed successfully"
}

# Send backup notification
send_backup_notification() {
    local backup_file="$1"
    local status="$2" # success or failure
    local webhook_url="${BACKUP_WEBHOOK_URL:-}"

    if [[ -z "$webhook_url" ]]; then
        log_message "INFO" "No webhook URL configured, skipping notification"
        return 0
    fi

    local backup_size=$(stat -f%z "$backup_file" 2>/dev/null || stat -c%s "$backup_file")
    local message

    if [[ "$status" == "success" ]]; then
        message="✅ **BitQuan Backup Successful**
Backup: $(basename "$backup_file")
Size: $((backup_size / 1024 / 1024))MB
Date: $(date)"
    else
        message="❌ **BitQuan Backup Failed**
Error: $status
Date: $(date)"
    fi

    curl -X POST "$webhook_url" \
        -H "Content-Type: application/json" \
        -d "{\"text\":\"$message\"}" \
        2>/dev/null || log_message "WARNING" "Failed to send backup notification"
}

# Cleanup on exit
cleanup() {
    if [[ -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi

    if [[ "$BACKUP_SUCCESS" != "true" ]]; then
        log_message "ERROR" "Backup process failed"
        send_backup_notification "" "failure"
    fi
}

# Main backup function
main() {
    log_message "INFO" "Starting BitQuan backup process..."
    log_message "INFO" "Backup type: $BACKUP_TYPE"

    # Set up cleanup trap
    trap cleanup EXIT

    # Initialize
    setup_logging
    check_prerequisites

    # Stop service for consistent backup
    stop_bitquan_service

    # Create backup components
    backup_configuration

    if [[ "$BACKUP_TYPE" != "config-only" ]]; then
        backup_blockchain_data
    fi

    backup_system_state
    backup_metadata

    # Compress and encrypt
    local compressed_file
    compressed_file=$(compress_backup)

    local final_file
    final_file=$(encrypt_backup "$compressed_file")

    # Verify backup
    verify_backup "$final_file"

    # Cleanup and maintenance
    cleanup_old_backups
    create_latest_symlink "$final_file"

    # Mark as successful
    BACKUP_SUCCESS=true
    log_message "INFO" "Backup process completed successfully"

    # Send notification
    send_backup_notification "$final_file" "success"

    # Start service again
    start_bitquan_service || {
        log_message "ERROR" "Failed to restart BitQuan service after backup"
        exit 1
    }

    # Display summary
    local final_size=$(stat -f%z "$final_file" 2>/dev/null || stat -c%s "$final_file")
    echo ""
    echo "🎉 Backup Summary:"
    echo "  File: $final_file"
    echo "  Size: $((final_size / 1024 / 1024))MB"
    echo "  Type: $BACKUP_TYPE"
    echo "  Date: $(date)"
    echo ""
}

# Show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

BitQuan Backup Script

OPTIONS:
    -t, --type TYPE          Backup type: full, incremental, config-only (default: full)
    -d, --dir DIRECTORY     Backup directory (default: $BACKUP_DIR)
    -c, --config DIRECTORY  Configuration directory (default: $CONFIG_DIR)
    -D, --data DIRECTORY    Data directory (default: $DATA_DIR)
    -r, --retention DAYS     Retention period in days (default: $RETENTION_DAYS)
    --no-compress           Disable compression
    --no-encrypt            Disable encryption
    -w, --webhook URL       Webhook URL for notifications
    -h, --help              Show this help message

EXAMPLES:
    # Full backup with default settings
    $0

    # Config-only backup
    $0 --type config-only

    # Incremental backup to custom directory
    $0 --type incremental --dir /mnt/backups

    # Backup without compression or encryption
    $0 --no-compress --no-encrypt

    # Backup with custom retention
    $0 --retention 60

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--type)
            BACKUP_TYPE="$2"
            shift 2
            ;;
        -d|--dir)
            BACKUP_DIR="$2"
            shift 2
            ;;
        -c|--config)
            CONFIG_DIR="$2"
            shift 2
            ;;
        -D|--data)
            DATA_DIR="$2"
            shift 2
            ;;
        -r|--retention)
            RETENTION_DAYS="$2"
            shift 2
            ;;
        --no-compress)
            COMPRESS=false
            shift
            ;;
        --no-encrypt)
            ENCRYPT=false
            shift
            ;;
        -w|--webhook)
            BACKUP_WEBHOOK_URL="$2"
            shift 2
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Validate backup type
case "$BACKUP_TYPE" in
    full|incremental|config-only)
        ;;
    *)
        echo "Invalid backup type: $BACKUP_TYPE"
        show_usage
        exit 1
        ;;
esac

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    log_message "WARNING" "Running as root is not recommended"
fi

# Start backup process
main "$@"

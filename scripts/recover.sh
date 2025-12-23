#!/bin/bash

# BitQuan Recovery Script
# ระบบ recover และ restore จาก backup

set -euo pipefail

# Configuration
BACKUP_DIR="${BACKUP_DIR:-/opt/backups/bitquan}"
CONFIG_DIR="${CONFIG_DIR:-/etc/bitquan}"
DATA_DIR="${DATA_DIR:-/var/lib/bitquan}"
LOG_FILE="${LOG_FILE:-/var/log/bitquan/recovery.log}"
TEMP_DIR="/tmp/bitquan_recovery_$(date +%s)"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Global variables
RECOVERY_SUCCESS=false
GPG_RECIPIENT="${GPG_RECIPIENT:-backup@bitquan.org}"

# Initialize logging
setup_logging() {
    mkdir -p "$(dirname "$LOG_FILE")"
    mkdir -p "$TEMP_DIR"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] Recovery process started" >> "$LOG_FILE"
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
    log_message "INFO" "Checking recovery prerequisites..."

    local required_commands=("tar" "gzip" "gpg" "cp" "rm")
    for cmd in "${required_commands[@]}"; do
        if ! command -v "$cmd" &> /dev/null; then
            log_message "ERROR" "Required command not found: $cmd"
            exit 1
        fi
    done

    # Check backup directory
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_message "ERROR" "Backup directory not found: $BACKUP_DIR"
        exit 1
    fi

    log_message "INFO" "Prerequisites check completed successfully"
}

# List available backups
list_backups() {
    log_message "INFO" "Listing available backups..."

    echo ""
    echo "📦 Available Backups:"
    echo "===================="

    local backup_count=0
    find "$BACKUP_DIR" -name "bitquan_backup_*.tar.gz*" -type f | sort -r | while read -r backup_file; do
        local filename=$(basename "$backup_file")
        local file_size=$(stat -f%z "$backup_file" 2>/dev/null || stat -c%s "$backup_file")
        local file_size_mb=$((file_size / 1024 / 1024))
        local file_date=$(stat -f%Sm "$backup_file" 2>/dev/null || stat -c%y "$backup_file")

        # Extract backup type from filename or metadata
        local backup_type="unknown"
        if [[ "$filename" == *"incremental"* ]]; then
            backup_type="incremental"
        elif [[ "$filename" == *"config"* ]]; then
            backup_type="config-only"
        else
            backup_type="full"
        fi

        echo "  📁 $filename"
        echo "     Type: $backup_type"
        echo "     Size: ${file_size_mb}MB"
        echo "     Date: $file_date"
        echo ""
        ((backup_count++))
    done

    if [[ $backup_count -eq 0 ]]; then
        log_message "WARNING" "No backup files found in $BACKUP_DIR"
        return 1
    fi

    log_message "INFO" "Found $backup_count backup files"
    return 0
}

# Select backup for recovery
select_backup() {
    local backup_pattern="$1"

    if [[ -n "$backup_pattern" ]]; then
        # Try to find exact match first
        local exact_match="$BACKUP_DIR/$backup_pattern"
        if [[ -f "$exact_match" ]]; then
            echo "$exact_match"
            return 0
        fi

        # Try pattern matching
        local matched_file=$(find "$BACKUP_DIR" -name "*${backup_pattern}*" -type f | head -1)
        if [[ -n "$matched_file" ]]; then
            echo "$matched_file"
            return 0
        fi

        log_message "ERROR" "No backup found matching pattern: $backup_pattern"
        return 1
    fi

    # Use latest backup if no pattern specified
    local latest_backup=$(find "$BACKUP_DIR" -name "bitquan_backup_*.tar.gz*" -type f -printf '%T@ %p\n' | sort -nr | head -1 | cut -d' ' -f2-)
    if [[ -n "$latest_backup" ]]; then
        echo "$latest_backup"
        return 0
    fi

    log_message "ERROR" "No backup files found"
    return 1
}

# Decrypt backup
decrypt_backup() {
    local encrypted_file="$1"
    local decrypted_file="$2"

    log_message "INFO" "Decrypting backup: $(basename "$encrypted_file")"

    gpg --batch --yes --decrypt --output "$decrypted_file" "$encrypted_file" || {
        log_message "ERROR" "Backup decryption failed"
        return 1
    }

    # Verify decrypted file
    if [[ ! -f "$decrypted_file" ]]; then
        log_message "ERROR" "Decrypted backup file not created"
        return 1
    fi

    log_message "INFO" "Backup decrypted successfully"
}

# Extract backup
extract_backup() {
    local backup_file="$1"
    local extract_dir="$2"

    log_message "INFO" "Extracting backup to: $extract_dir"

    mkdir -p "$extract_dir"

    tar -xzf "$backup_file" -C "$extract_dir" || {
        log_message "ERROR" "Backup extraction failed"
        return 1
    }

    # Verify extraction
    if [[ ! -d "$extract_dir/config" ]]; then
        log_message "ERROR" "Backup extraction failed - config directory not found"
        return 1
    fi

    log_message "INFO" "Backup extracted successfully"
}

# Validate backup metadata
validate_backup() {
    local extract_dir="$1"

    log_message "INFO" "Validating backup metadata..."

    local metadata_file="$extract_dir/metadata.json"
    if [[ ! -f "$metadata_file" ]]; then
        log_message "ERROR" "Backup metadata file not found"
        return 1
    fi

    # Read metadata
    local hostname=$(jq -r '.hostname' "$metadata_file" 2>/dev/null || echo "unknown")
    local backup_date=$(jq -r '.backup_date' "$metadata_file" 2>/dev/null || echo "unknown")
    local git_commit=$(jq -r '.git_commit' "$metadata_file" 2>/dev/null || echo "unknown")
    local backup_type=$(jq -r '.backup_type' "$metadata_file" 2>/dev/null || echo "unknown")

    log_message "INFO" "Backup Details:"
    log_message "INFO" "  Hostname: $hostname"
    log_message "INFO" "  Backup Date: $backup_date"
    log_message "INFO" "  Git Commit: $git_commit"
    log_message "INFO" "  Backup Type: $backup_type"

    # Check hostname compatibility
    local current_hostname=$(hostname)
    if [[ "$hostname" != "$current_hostname" && "$hostname" != "unknown" ]]; then
        log_message "WARNING" "Backup hostname ($hostname) differs from current hostname ($current_hostname)"
        read -p "Continue anyway? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_message "INFO" "Recovery cancelled by user"
            return 1
        fi
    fi

    log_message "INFO" "Backup validation completed successfully"
    return 0
}

# Stop BitQuan service
stop_bitquan_service() {
    log_message "INFO" "Stopping BitQuan service for recovery..."

    if systemctl is-active --quiet bitquan 2>/dev/null; then
        systemctl stop bitquan || {
            log_message "WARNING" "Failed to stop BitQuan service with systemctl"
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
            return 1
        fi
    else
        log_message "INFO" "BitQuan service is not running"
    fi

    log_message "INFO" "BitQuan service stopped successfully"
}

# Create backup of current state
backup_current_state() {
    log_message "INFO" "Creating backup of current state before recovery..."

    local backup_name="pre_recovery_$(date +%Y%m%d_%H%M%S)"
    local current_backup="$BACKUP_DIR/${backup_name}.tar.gz"

    # Create temporary directory
    local temp_backup_dir="/tmp/pre_recovery_$$"
    mkdir -p "$temp_backup_dir"

    # Backup current configuration
    if [[ -d "$CONFIG_DIR" ]]; then
        cp -r "$CONFIG_DIR" "$temp_backup_dir/" 2>/dev/null || true
    fi

    # Create metadata
    cat > "$temp_backup_dir/metadata.json" << EOF
{
    "backup_type": "pre-recovery",
    "backup_date": "$(date -Iseconds)",
    "backup_name": "$backup_name",
    "hostname": "$(hostname)",
    "reason": "Automatic backup before recovery operation"
}
EOF

    # Compress backup
    tar -czf "$current_backup" -C "$temp_backup_dir" . || {
        log_message "WARNING" "Failed to create pre-recovery backup"
    }

    # Cleanup
    rm -rf "$temp_backup_dir"

    if [[ -f "$current_backup" ]]; then
        log_message "INFO" "Pre-recovery backup created: $current_backup"
    fi
}

# Restore configuration
restore_configuration() {
    local backup_config_dir="$1"

    log_message "INFO" "Restoring BitQuan configuration..."

    # Backup existing configuration
    if [[ -d "$CONFIG_DIR" ]]; then
        local config_backup="${CONFIG_DIR}.backup.$(date +%Y%m%d_%H%M%S)"
        log_message "INFO" "Backing up existing configuration to: $config_backup"
        mv "$CONFIG_DIR" "$config_backup" || {
            log_message "ERROR" "Failed to backup existing configuration"
            return 1
        }
    fi

    # Restore configuration
    if [[ -d "$backup_config_dir" ]]; then
        cp -r "$backup_config_dir" "$CONFIG_DIR" || {
            log_message "ERROR" "Failed to restore configuration"
            return 1
        }

        # Set proper permissions
        chown -R bitquan:bitquan "$CONFIG_DIR" 2>/dev/null || {
            log_message "WARNING" "Failed to set ownership of configuration directory"
        }

        chmod -R 644 "$CONFIG_DIR" 2>/dev/null || true
        find "$CONFIG_DIR" -type d -exec chmod 755 {} \; 2>/dev/null || true
    else
        log_message "WARNING" "No configuration found in backup"
        return 1
    fi

    log_message "INFO" "Configuration restored successfully"
}

# Restore blockchain data
restore_blockchain_data() {
    local backup_data_dir="$1"

    if [[ ! -d "$backup_data_dir" ]]; then
        log_message "INFO" "No blockchain data in backup, skipping data restoration"
        return 0
    fi

    log_message "INFO" "Restoring blockchain data..."

    # Backup existing data
    if [[ -d "$DATA_DIR" ]]; then
        local data_backup="${DATA_DIR}.backup.$(date +%Y%m%d_%H%M%S)"
        log_message "INFO" "Backing up existing data to: $data_backup"
        mv "$DATA_DIR" "$data_backup" || {
            log_message "ERROR" "Failed to backup existing data"
            return 1
        }
    fi

    # Create data directory
    mkdir -p "$DATA_DIR"

    # Restore data
    cp -r "$backup_data_dir/"* "$DATA_DIR/" || {
        log_message "ERROR" "Failed to restore blockchain data"
        return 1
    }

    # Verify data integrity
    if [[ -f "$backup_data_dir/sha256sums.txt" ]]; then
        log_message "INFO" "Verifying data integrity..."
        cd "$DATA_DIR"
        sha256sum -c "$backup_data_dir/sha256sums.txt" || {
            log_message "WARNING" "Data integrity check failed"
        }
        cd - > /dev/null
    fi

    # Set proper permissions
    chown -R bitquan:bitquan "$DATA_DIR" 2>/dev/null || {
        log_message "WARNING" "Failed to set ownership of data directory"
    }

    log_message "INFO" "Blockchain data restored successfully"
}

# Restore systemd service files
restore_services() {
    local backup_config_dir="$1"

    log_message "INFO" "Restoring systemd service files..."

    # Find and restore service files
    find "$backup_config_dir" -name "*.service" -type f | while read -r service_file; do
        local service_name=$(basename "$service_file")
        local service_path="/etc/systemd/system/$service_name"

        log_message "INFO" "Restoring service: $service_name"
        cp "$service_file" "$service_path" || {
            log_message "WARNING" "Failed to restore service file: $service_name"
            continue
        }

        # Reload systemd
        systemctl daemon-reload || {
            log_message "WARNING" "Failed to reload systemd daemon"
        }
    done

    log_message "INFO" "Systemd service files restored"
}

# Verify restoration
verify_restoration() {
    log_message "INFO" "Verifying restoration..."

    # Check configuration
    if [[ ! -d "$CONFIG_DIR" ]]; then
        log_message "ERROR" "Configuration directory not restored"
        return 1
    fi

    # Check for essential configuration files
    local essential_files=("bitquan.conf" "bitquan.toml" "wallet.conf")
    for file in "${essential_files[@]}"; do
        if [[ -f "$CONFIG_DIR/$file" ]]; then
            log_message "INFO" "✓ Configuration file found: $file"
        else
            log_message "WARNING" "Configuration file missing: $file"
        fi
    done

    # Check data directory
    if [[ -d "$DATA_DIR" ]]; then
        log_message "INFO" "✓ Data directory exists"
    else
        log_message "WARNING" "Data directory not found"
    fi

    # Check service files
    if systemctl list-unit-files | grep -q bitquan; then
        log_message "INFO" "✓ BitQuan service files found"
    else
        log_message "WARNING" "BitQuan service files not found"
    fi

    log_message "INFO" "Restoration verification completed"
}

# Start BitQuan service
start_bitquan_service() {
    log_message "INFO" "Starting BitQuan service..."

    # Enable service
    systemctl enable bitquan || {
        log_message "WARNING" "Failed to enable BitQuan service"
    }

    # Start service
    systemctl start bitquan || {
        log_message "ERROR" "Failed to start BitQuan service"
        return 1
    }

    # Wait for service to start
    local timeout=60
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
}

# Cleanup on exit
cleanup() {
    if [[ -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
    fi

    if [[ "$RECOVERY_SUCCESS" != "true" ]]; then
        log_message "ERROR" "Recovery process failed"
    fi
}

# Show recovery status
show_recovery_status() {
    echo ""
    echo "🎉 Recovery Status:"
    echo "==================="
    echo ""

    # Service status
    if systemctl is-active --quiet bitquan 2>/dev/null; then
        echo "✅ BitQuan Service: Running"
    else
        echo "❌ BitQuan Service: Not running"
    fi

    # Configuration status
    if [[ -d "$CONFIG_DIR" ]]; then
        echo "✅ Configuration: Restored"
        echo "   Location: $CONFIG_DIR"
    else
        echo "❌ Configuration: Not found"
    fi

    # Data status
    if [[ -d "$DATA_DIR" ]]; then
        local data_size=$(du -sh "$DATA_DIR" 2>/dev/null | cut -f1 || echo "unknown")
        echo "✅ Blockchain Data: Restored"
        echo "   Location: $DATA_DIR"
        echo "   Size: $data_size"
    else
        echo "⚠️  Blockchain Data: Not found (config-only backup)"
    fi

    echo ""
    echo "📊 System Information:"
    echo "  Uptime: $(uptime -p 2>/dev/null || uptime)"
    echo "  Load: $(uptime | awk -F'load average:' '{print $2}' | cut -d',' -f1 | xargs)"
    echo "  Memory: $(free -h | grep Mem | awk '{printf "%s/%s (%.0f%%)", $3,$2,$3/$2*100.0}')"
    echo ""

    # Recent logs
    if [[ -f "$LOG_FILE" ]]; then
        echo "📋 Recent Recovery Logs:"
        tail -5 "$LOG_FILE"
    fi
}

# Send recovery notification
send_recovery_notification() {
    local status="$1" # success or failure
    local webhook_url="${RECOVERY_WEBHOOK_URL:-}"

    if [[ -z "$webhook_url" ]]; then
        return 0
    fi

    local message
    if [[ "$status" == "success" ]]; then
        message="✅ **BitQuan Recovery Successful**
System: $(hostname)
Date: $(date)"
Status: Services restored and running"
    else
        message="❌ **BitQuan Recovery Failed**
System: $(hostname)
Date: $(date)
Status: Recovery process encountered errors"
    fi

    curl -X POST "$webhook_url" \
        -H "Content-Type: application/json" \
        -d "{\"text\":\"$message\"}" \
        2>/dev/null || log_message "WARNING" "Failed to send recovery notification"
}

# Main recovery function
main() {
    local backup_pattern="$1"
    local force_mode="$2"

    log_message "INFO" "Starting BitQuan recovery process..."

    # Set up cleanup trap
    trap cleanup EXIT

    # Initialize
    setup_logging
    check_prerequisites

    # List available backups
    if [[ -z "$backup_pattern" ]]; then
        list_backups
        echo ""
        read -p "Enter backup name or pattern (or press Enter for latest): " backup_pattern
        echo ""
    fi

    # Select backup
    local backup_file
    backup_file=$(select_backup "$backup_pattern") || {
        log_message "ERROR" "No suitable backup found for recovery"
        exit 1
    }

    log_message "INFO" "Selected backup: $(basename "$backup_file")"

    # Confirm recovery
    if [[ "$force_mode" != "--force" ]]; then
        echo "⚠️  This will restore BitQuan from backup and may overwrite current data."
        echo "   Backup: $(basename "$backup_file")"
        echo ""
        read -p "Continue with recovery? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_message "INFO" "Recovery cancelled by user"
            exit 0
        fi
    fi

    # Decrypt backup if encrypted
    local decrypted_file="$TEMP_DIR/backup.tar.gz"
    if [[ "$backup_file" == *.gpg ]]; then
        decrypt_backup "$backup_file" "$decrypted_file" || {
            log_message "ERROR" "Failed to decrypt backup"
            exit 1
        }
    else
        cp "$backup_file" "$decrypted_file"
    fi

    # Extract backup
    local extract_dir="$TEMP_DIR/extracted"
    extract_backup "$decrypted_file" "$extract_dir" || {
        log_message "ERROR" "Failed to extract backup"
        exit 1
    }

    # Validate backup
    validate_backup "$extract_dir" || {
        log_message "ERROR" "Backup validation failed"
        exit 1
    }

    # Stop service
    stop_bitquan_service || {
        log_message "ERROR" "Failed to stop BitQuan service"
        exit 1
    }

    # Create pre-recovery backup
    backup_current_state

    # Restore components
    restore_configuration "$extract_dir/config" || {
        log_message "ERROR" "Failed to restore configuration"
        exit 1
    }

    if [[ -d "$extract_dir/data" ]]; then
        restore_blockchain_data "$extract_dir/data" || {
            log_message "ERROR" "Failed to restore blockchain data"
            exit 1
        }
    fi

    restore_services "$extract_dir/config" || {
        log_message "WARNING" "Failed to restore some service files"
    }

    # Verify restoration
    verify_restoration || {
        log_message "WARNING" "Some verification checks failed"
    }

    # Start service
    start_bitquan_service || {
        log_message "ERROR" "Failed to start BitQuan service after recovery"
        exit 1
    }

    # Mark as successful
    RECOVERY_SUCCESS=true
    log_message "INFO" "Recovery process completed successfully"

    # Show status
    show_recovery_status

    # Send notification
    send_recovery_notification "success"

    echo ""
    echo "🎉 BitQuan recovery completed successfully!"
    echo "   Check logs: journalctl -u bitquan -f"
    echo "   Service status: systemctl status bitquan"
}

# Show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS] [BACKUP_PATTERN]

BitQuan Recovery Script

ARGUMENTS:
    BACKUP_PATTERN          Backup file name or pattern to restore from

OPTIONS:
    -f, --force             Skip confirmation prompts
    -d, --backup-dir DIR     Backup directory (default: $BACKUP_DIR)
    -c, --config DIR         Configuration directory (default: $CONFIG_DIR)
    -D, --data DIR           Data directory (default: $DATA_DIR)
    -l, --list               List available backups and exit
    -h, --help              Show this help message

EXAMPLES:
    # List available backups
    $0 --list

    # Restore from latest backup
    $0

    # Restore from specific backup
    $0 bitquan_backup_20241218_120000.tar.gz.gpg

    # Force restore without confirmation
    $0 --force bitquan_backup_20241218_120000.tar.gz.gpg

EOF
}

# Parse command line arguments
FORCE_MODE=""
BACKUP_PATTERN=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -f|--force)
            FORCE_MODE="--force"
            shift
            ;;
        -d|--backup-dir)
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
        -l|--list)
            list_backups
            exit 0
            ;;
        -h|--help)
            show_usage
            exit 0
            ;;
        -*)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
        *)
            BACKUP_PATTERN="$1"
            shift
            ;;
    esac
done

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    log_message "WARNING" "Running as root is not recommended"
fi

# Start recovery process
main "$BACKUP_PATTERN" "$FORCE_MODE"

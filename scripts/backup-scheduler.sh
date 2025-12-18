#!/bin/bash

# BitQuan Backup Scheduler
# จัดการ schedule และ manage automated backups

set -euo pipefail

# Configuration
BACKUP_SCRIPT="${BACKUP_SCRIPT:-/opt/bitquan/scripts/backup.sh}"
LOG_FILE="${LOG_FILE:-/var/log/bitquan/backup-scheduler.log}"
CONFIG_FILE="${CONFIG_FILE:-/etc/bitquan/backup-scheduler.conf}"
PID_FILE="${PID_FILE:-/var/run/bitquan-backup-scheduler.pid}"

# Default schedule settings
DAILY_BACKUP_HOUR="${DAILY_BACKUP_HOUR:-2}"        # 2 AM
DAILY_BACKUP_MINUTE="${DAILY_BACKUP_MINUTE:-0}"    # 0 minutes
WEEKLY_FULL_BACKUP_DAY="${WEEKLY_FULL_BACKUP_DAY:-0}" # Sunday (0=Sunday)
HOURLY_INCREMENTAL="${HOURLY_INCREMENTAL:-true}"
CONFIG_CHANGE_DETECTION="${CONFIG_CHANGE_DETECTION:-true}"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Global variables
SCHEDULER_RUNNING=false
BACKUP_IN_PROGRESS=false

# Initialize logging
setup_logging() {
    mkdir -p "$(dirname "$LOG_FILE")"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] Backup scheduler started" >> "$LOG_FILE"
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

# Create default configuration
create_default_config() {
    cat > "$CONFIG_FILE" << EOF
# BitQuan Backup Scheduler Configuration
# Generated on $(date)

# Backup Types and Schedules
DAILY_BACKUP_TYPE="full"
INCREMENTAL_BACKUP_ENABLED="true"
HOURLY_INCREMENTAL_BACKUP="true"

# Schedule Settings
DAILY_BACKUP_HOUR="2"
DAILY_BACKUP_MINUTE="0"
WEEKLY_FULL_BACKUP_DAY="0"  # 0=Sunday, 1=Monday, ..., 6=Saturday
MONTHLY_BACKUP_DAY="1"       # 1st day of month

# Retention Policies
DAILY_RETENTION_DAYS="7"
WEEKLY_RETENTION_WEEKS="4"
MONTHLY_RETENTION_MONTHS="12"
CONFIG_BACKUP_RETENTION_DAYS="30"

# Backup Options
COMPRESS_BACKUPS="true"
ENCRYPT_BACKUPS="true"
VERIFY_BACKUPS="true"
PRE_BACKUP_HOOK="/opt/bitquan/scripts/pre-backup.sh"
POST_BACKUP_HOOK="/opt/bitquan/scripts/post-backup.sh"

# Notification Settings
ENABLE_NOTIFICATIONS="true"
NOTIFICATION_WEBHOOK="${NOTIFICATION_WEBHOOK_URL:-}"
EMAIL_NOTIFICATIONS="false"
EMAIL_RECIPIENTS="admin@bitquan.org"

# Resource Management
MAX_PARALLEL_BACKUPS="1"
CPU_LIMIT="50"
MEMORY_LIMIT="2048"  # MB
DISK_SPACE_THRESHOLD="90"  # Percent

# Monitoring and Health Checks
HEALTH_CHECK_ENABLED="true"
HEALTH_CHECK_INTERVAL="300"  # Seconds
RECOVERY_POINT_VALIDATION="true"

# Advanced Settings
BACKUP_ENCRYPTION_KEY="backup@bitquan.org"
BACKUP_COMPRESSION_LEVEL="6"
BACKUP_CHUNK_SIZE="1024"  # MB
EOF

    log_message "INFO" "Created default configuration: $CONFIG_FILE"
}

# Load configuration
load_config() {
    if [[ ! -f "$CONFIG_FILE" ]]; then
        log_message "INFO" "Configuration file not found, creating default"
        create_default_config
    fi

    # Source configuration
    source "$CONFIG_FILE" || {
        log_message "ERROR" "Failed to load configuration file: $CONFIG_FILE"
        exit 1
    }

    log_message "INFO" "Configuration loaded from: $CONFIG_FILE"
}

# Detect configuration changes
detect_config_changes() {
    if [[ "$CONFIG_CHANGE_DETECTION" != "true" ]]; then
        return 0
    fi

    local config_hash_file="/tmp/bitquan_config_hash"
    local current_hash=$(sha256sum "$CONFIG_FILE" | cut -d' ' -f1)

    if [[ -f "$config_hash_file" ]]; then
        local previous_hash=$(cat "$config_hash_file")
        if [[ "$current_hash" != "$previous_hash" ]]; then
            log_message "INFO" "Configuration changed, creating backup"
            execute_backup "config-only" "Configuration change detected"
        fi
    fi

    echo "$current_hash" > "$config_hash_file"
}

# Check system resources
check_system_resources() {
    # Check disk space
    local disk_usage=$(df "$BACKUP_DIR" 2>/dev/null | tail -1 | awk '{print $5}' | tr -d '%')
    local disk_usage_num=${disk_usage%.*}

    if [[ "$disk_usage_num" -gt "${DISK_SPACE_THRESHOLD:-90}" ]]; then
        log_message "WARNING" "Disk usage high: ${disk_usage}% (threshold: ${DISK_SPACE_THRESHOLD:-90}%)"

        # Trigger cleanup if available
        if command -v /opt/bitquan/scripts/backup-cleanup.sh &> /dev/null; then
            log_message "INFO" "Running backup cleanup due to high disk usage"
            /opt/bitquan/scripts/backup-cleanup.sh
        fi
    fi

    # Check memory usage
    local mem_usage=$(free | grep Mem | awk '{printf "%.0f", $3/$2 * 100.0}')
    if [[ "$mem_usage" -gt 80 ]]; then
        log_message "WARNING" "High memory usage: ${mem_usage}%"
    fi

    # Check CPU usage
    local cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1 2>/dev/null || echo "0")
    if (( $(echo "$cpu_usage > 80" | bc -l) )); then
        log_message "WARNING" "High CPU usage: ${cpu_usage}%"
    fi
}

# Execute backup
execute_backup() {
    local backup_type="$1"
    local reason="$2"

    if [[ "$BACKUP_IN_PROGRESS" == "true" ]]; then
        log_message "WARNING" "Backup already in progress, skipping: $backup_type"
        return 1
    fi

    BACKUP_IN_PROGRESS=true
    log_message "INFO" "Starting $backup_type backup: $reason"

    local backup_start=$(date +%s)

    # Run pre-backup hook
    if [[ -n "${PRE_BACKUP_HOOK:-}" && -x "$PRE_BACKUP_HOOK" ]]; then
        log_message "INFO" "Running pre-backup hook: $PRE_BACKUP_HOOK"
        "$PRE_BACKUP_HOOK" || log_message "WARNING" "Pre-backup hook failed"
    fi

    # Execute backup
    local backup_cmd="$BACKUP_SCRIPT --type $backup_type"
    if [[ "$COMPRESS_BACKUPS" == "true" ]]; then
        backup_cmd="$backup_cmd --compress"
    fi
    if [[ "$ENCRYPT_BACKUPS" == "true" ]]; then
        backup_cmd="$backup_cmd --encrypt"
    fi

    if eval "$backup_cmd"; then
        local backup_end=$(date +%s)
        local backup_duration=$((backup_end - backup_start))

        log_message "INFO" "$backup_type backup completed successfully (${backup_duration}s)"

        # Run post-backup hook
        if [[ -n "${POST_BACKUP_HOOK:-}" && -x "$POST_BACKUP_HOOK" ]]; then
            log_message "INFO" "Running post-backup hook: $POST_BACKUP_HOOK"
            "$POST_BACKUP_HOOK" || log_message "WARNING" "Post-backup hook failed"
        fi

        # Verify backup if enabled
        if [[ "$VERIFY_BACKUPS" == "true" ]]; then
            verify_latest_backup
        fi

        # Send notification
        send_backup_notification "$backup_type" "success" "$reason"

        BACKUP_IN_PROGRESS=false
        return 0
    else
        log_message "ERROR" "$backup_type backup failed"
        send_backup_notification "$backup_type" "failure" "$reason"
        BACKUP_IN_PROGRESS=false
        return 1
    fi
}

# Verify latest backup
verify_latest_backup() {
    log_message "INFO" "Verifying latest backup..."

    local latest_backup=$(find "$BACKUP_DIR" -name "bitquan_backup_*.tar.gz*" -type f -printf '%T@ %p\n' | sort -nr | head -1 | cut -d' ' -f2-)

    if [[ -n "$latest_backup" && -f "$latest_backup" ]]; then
        log_message "INFO" "Latest backup: $(basename "$latest_backup")"

        # Check file size
        local file_size=$(stat -f%z "$latest_backup" 2>/dev/null || stat -c%s "$latest_backup")
        if [[ "$file_size" -gt 0 ]]; then
            log_message "INFO" "Backup file size: $((file_size / 1024 / 1024))MB"
        else
            log_message "ERROR" "Backup file is empty"
            return 1
        fi

        # Verify encryption if enabled
        if [[ "$ENCRYPT_BACKUPS" == "true" && "$latest_backup" == *.gpg ]]; then
            log_message "INFO" "Backup appears to be encrypted"
        fi

        log_message "INFO" "Backup verification completed successfully"
        return 0
    else
        log_message "WARNING" "No backup files found for verification"
        return 1
    fi
}

# Send backup notification
send_backup_notification() {
    local backup_type="$1"
    local status="$2"
    local reason="$3"
    local webhook_url="${NOTIFICATION_WEBHOOK:-}"

    if [[ "$ENABLE_NOTIFICATIONS" != "true" || -z "$webhook_url" ]]; then
        return 0
    fi

    local message
    local emoji="🔄"

    case "$backup_type" in
        "full")
            emoji="💾"
            ;;
        "incremental")
            emoji="📝"
            ;;
        "config-only")
            emoji="⚙️"
            ;;
    esac

    if [[ "$status" == "success" ]]; then
        message="$emoji **BitQuan $backup_type Backup Successful**"
        message="$message\\nReason: $reason"
        message="$message\\nTime: $(date)"
    else
        message="❌ **BitQuan $backup_type Backup Failed**"
        message="$message\\nReason: $reason"
        message="$message\\nTime: $(date)"
    fi

    curl -X POST "$webhook_url" \
        -H "Content-Type: application/json" \
        -d "{\"text\":\"$message\"}" \
        2>/dev/null || log_message "WARNING" "Failed to send notification"
}

# Cleanup old backups
cleanup_old_backups() {
    log_message "INFO" "Starting backup cleanup..."

    # Daily backup cleanup
    if [[ -n "${DAILY_RETENTION_DAYS:-}" ]]; then
        find "$BACKUP_DIR" -name "bitquan_backup_*_full.tar.gz*" -type f \
            -mtime +$DAILY_RETENTION_DAYS -delete || \
            log_message "WARNING" "Some daily backup files could not be deleted"
    fi

    # Weekly backup cleanup
    if [[ -n "${WEEKLY_RETENTION_WEEKS:-}" ]]; then
        local weekly_days=$((WEEKLY_RETENTION_WEEKS * 7))
        find "$BACKUP_DIR" -name "*weekly*.tar.gz*" -type f \
            -mtime +$weekly_days -delete || \
            log_message "WARNING" "Some weekly backup files could not be deleted"
    fi

    # Monthly backup cleanup
    if [[ -n "${MONTHLY_RETENTION_MONTHS:-}" ]]; then
        local monthly_days=$((MONTHLY_RETENTION_MONTHS * 30))
        find "$BACKUP_DIR" -name "*monthly*.tar.gz*" -type f \
            -mtime +$monthly_days -delete || \
            log_message "WARNING" "Some monthly backup files could not be deleted"
    fi

    # Config backup cleanup
    if [[ -n "${CONFIG_BACKUP_RETENTION_DAYS:-}" ]]; then
        find "$BACKUP_DIR" -name "*config*.tar.gz*" -type f \
            -mtime +$CONFIG_BACKUP_RETENTION_DAYS -delete || \
            log_message "WARNING" "Some config backup files could not be deleted"
    fi

    log_message "INFO" "Backup cleanup completed"
}

# Health check function
health_check() {
    if [[ "$HEALTH_CHECK_ENABLED" != "true" ]]; then
        return 0
    fi

    log_message "DEBUG" "Running health check..."

    # Check backup script exists and is executable
    if [[ ! -x "$BACKUP_SCRIPT" ]]; then
        log_message "ERROR" "Backup script not found or not executable: $BACKUP_SCRIPT"
        return 1
    fi

    # Check backup directory exists
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_message "ERROR" "Backup directory not found: $BACKUP_DIR"
        return 1
    fi

    # Check recent backup exists
    local recent_backup=$(find "$BACKUP_DIR" -name "bitquan_backup_*.tar.gz*" -type f -mtime -1 | wc -l)
    if [[ "$recent_backup" -eq 0 ]]; then
        log_message "WARNING" "No recent backup found (within 24 hours)"
        return 1
    fi

    log_message "DEBUG" "Health check completed successfully"
    return 0
}

# Schedule management functions
should_run_daily_backup() {
    local current_hour=$(date +%H)
    local current_minute=$(date +%M)

    # Check if it's the right time
    if [[ "$current_hour" == "$DAILY_BACKUP_HOUR" && "$current_minute" == "$DAILY_BACKUP_MINUTE" ]]; then
        return 0
    else
        return 1
    fi
}

should_run_weekly_backup() {
    local current_day=$(date +%w)  # 0=Sunday, 1=Monday, ...

    if [[ "$current_day" == "$WEEKLY_FULL_BACKUP_DAY" ]]; then
        return 0
    else
        return 1
    fi
}

should_run_monthly_backup() {
    local current_day=$(date +%d)

    if [[ "$current_day" == "01" ]]; then
        return 0
    else
        return 1
    fi
}

should_run_hourly_incremental() {
    if [[ "$HOURLY_INCREMENTAL" != "true" ]]; then
        return 1
    fi

    local current_minute=$(date +%M)

    # Run at minute 0 of every hour
    if [[ "$current_minute" == "00" ]]; then
        return 0
    else
        return 1
    fi
}

should_run_config_change_backup() {
    detect_config_changes
}

# Main scheduler loop
scheduler_loop() {
    log_message "INFO" "Starting backup scheduler loop..."

    while [[ "$SCHEDULER_RUNNING" == "true" ]]; do
        local current_time=$(date '+%Y-%m-%d %H:%M:%S')

        log_message "DEBUG" "Checking schedules at $current_time"

        # Check resource usage
        check_system_resources

        # Health check
        health_check

        # Daily backup
        if should_run_daily_backup; then
            if should_run_weekly_backup; then
                # Weekly backup (full backup on specific day)
                execute_backup "full" "Weekly scheduled backup"
            else
                # Regular daily backup
                if [[ "$INCREMENTAL_BACKUP_ENABLED" == "true" ]]; then
                    execute_backup "incremental" "Daily incremental backup"
                else
                    execute_backup "full" "Daily full backup"
                fi
            fi
        fi

        # Hourly incremental backup
        if should_run_hourly_incremental && should_run_daily_backup; then
            execute_backup "incremental" "Hourly incremental backup"
        fi

        # Monthly backup
        if should_run_monthly_backup; then
            execute_backup "full" "Monthly full backup"
        fi

        # Configuration change backup
        should_run_config_change_backup

        # Daily cleanup (run at 3 AM)
        local current_hour=$(date +%H)
        if [[ "$current_hour" == "03" && $(date +%M) == "00" ]]; then
            cleanup_old_backups
        fi

        # Sleep until next minute
        sleep 60
    done
}

# Signal handlers
stop_scheduler() {
    log_message "INFO" "Received stop signal, shutting down scheduler..."
    SCHEDULER_RUNNING=false
}

reload_config() {
    log_message "INFO" "Received reload signal, reloading configuration..."
    load_config
    log_message "INFO" "Configuration reloaded successfully"
}

# Show status
show_status() {
    echo "BitQuan Backup Scheduler Status"
    echo "============================"
    echo ""
    echo "Running: $([ "$SCHEDULER_RUNNING" == "true" ] && echo "Yes" || echo "No")"
    echo "Backup in Progress: $BACKUP_IN_PROGRESS"
    echo "Config File: $CONFIG_FILE"
    echo "Log File: $LOG_FILE"
    echo "PID File: $PID_FILE"
    echo ""

    if [[ -f "$PID_FILE" ]]; then
        local pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            echo "Process ID: $pid"
            echo "Process Running: Yes"
        else
            echo "Process ID: $pid"
            echo "Process Running: No (stale PID file)"
        fi
    else
        echo "Process ID: Not running"
    fi

    echo ""
    echo "Configuration:"
    echo "  Daily Backup Hour: $DAILY_BACKUP_HOUR"
    echo "  Weekly Full Backup Day: $WEEKLY_FULL_BACKUP_DAY"
    echo "  Hourly Incremental: $HOURLY_INCREMENTAL"
    echo "  Config Change Detection: $CONFIG_CHANGE_DETECTION"
    echo ""

    echo "Recent Backups:"
    find "$BACKUP_DIR" -name "bitquan_backup_*.tar.gz*" -type f -mtime -7 -ls | head -5 || echo "  No recent backups found"
}

# Show usage
show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

BitQuan Backup Scheduler

OPTIONS:
    start           Start the backup scheduler daemon
    stop            Stop the backup scheduler daemon
    restart         Restart the backup scheduler daemon
    status          Show scheduler status
    reload           Reload configuration
    backup          Run immediate backup
    cleanup          Run backup cleanup
    verify          Verify latest backup
    config          Show current configuration
    test-schedules   Test backup schedule logic
    -h, --help       Show this help message

BACKUP OPTIONS:
    --type TYPE     Backup type: full, incremental, config-only
    --reason REASON Reason for backup

EXAMPLES:
    # Start scheduler
    $0 start

    # Show status
    $0 status

    # Run immediate backup
    $0 backup --type full --reason "Manual backup"

    # Test schedules
    $0 test-schedules

    # Reload configuration
    $0 reload

EOF
}

# Test backup schedule logic
test_schedules() {
    echo "Testing backup schedule logic..."
    echo "Current time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "Current hour: $(date +%H)"
    echo "Current minute: $(date +%M)"
    echo "Current day of week: $(date +%w) (0=Sunday)"
    echo "Current day of month: $(date +%d)"
    echo ""

    echo "Schedule Tests:"
    echo "=============="

    if should_run_daily_backup; then
        echo "✅ Daily backup should run now"
    else
        echo "❌ Daily backup should NOT run now"
    fi

    if should_run_weekly_backup; then
        echo "✅ Weekly backup should run now"
    else
        echo "❌ Weekly backup should NOT run now"
    fi

    if should_run_monthly_backup; then
        echo "✅ Monthly backup should run now"
    else
        echo "❌ Monthly backup should NOT run now"
    fi

    if should_run_hourly_incremental; then
        echo "✅ Hourly incremental backup should run now"
    else
        echo "❌ Hourly incremental backup should NOT run now"
    fi

    echo ""
    echo "Next backup times:"
    echo "=================="

    # Calculate next daily backup
    local next_daily=$(date -d "today $DAILY_BACKUP_HOUR:$DAILY_BACKUP_MINUTE:00" '+%Y-%m-%d %H:%M:%S')
    if [[ "$(date -d "$next_daily" +%s)" -lt "$(date +%s)" ]]; then
        next_daily=$(date -d "tomorrow $DAILY_BACKUP_HOUR:$DAILY_BACKUP_MINUTE:00" '+%Y-%m-%d %H:%M:%S')
    fi
    echo "Next daily backup: $next_daily"

    # Calculate next hourly backup
    local next_hourly=$(date -d "$(date +%Y-%m-%d %H):00:00" '+%Y-%m-%d %H:%M:%S')
    if [[ "$(date -d "$next_hourly" +%s)" -le "$(date +%s)" ]]; then
        next_hourly=$(date -d "$(date +%Y-%m-%d %H + 1 hour):00:00" '+%Y-%m-%d %H:%M:%S')
    fi
    echo "Next hourly backup: $next_hourly"
}

# Main execution logic
main() {
    local command="$1"
    shift

    # Initialize
    setup_logging

    case "$command" in
        "start")
            if [[ -f "$PID_FILE" ]]; then
                local pid=$(cat "$PID_FILE")
                if kill -0 "$pid" 2>/dev/null; then
                    log_message "ERROR" "Scheduler is already running (PID: $pid)"
                    exit 1
                else
                    log_message "WARNING" "Removing stale PID file"
                    rm -f "$PID_FILE"
                fi
            fi

            load_config

            echo "Starting BitQuan backup scheduler..."
            echo "PID will be written to: $PID_FILE"
            echo "Logs will be written to: $LOG_FILE"

            # Start scheduler in background
            SCHEDULER_RUNNING=true
            echo $$ > "$PID_FILE"
            scheduler_loop
            ;;

        "stop")
            if [[ -f "$PID_FILE" ]]; then
                local pid=$(cat "$PID_FILE")
                if kill -0 "$pid" 2>/dev/null; then
                    log_message "INFO" "Stopping scheduler (PID: $pid)"
                    kill "$pid"
                    rm -f "$PID_FILE"
                    echo "Scheduler stopped"
                else
                    log_message "WARNING" "Scheduler not running (stale PID file)"
                    rm -f "$PID_FILE"
                    echo "Scheduler not running"
                fi
            else
                echo "Scheduler not running"
            fi
            ;;

        "restart")
            $0 stop
            sleep 2
            $0 start
            ;;

        "status")
            show_status
            ;;

        "reload")
            if [[ -f "$PID_FILE" ]]; then
                local pid=$(cat "$PID_FILE")
                if kill -0 "$pid" 2>/dev/null; then
                    kill -HUP "$pid"
                    echo "Configuration reload signal sent to scheduler (PID: $pid)"
                else
                    echo "Scheduler not running"
                fi
            else
                echo "Scheduler not running"
            fi
            ;;

        "backup")
            load_config
            local backup_type="full"
            local reason="Manual backup"

            while [[ $# -gt 0 ]]; do
                case "$1" in
                    --type)
                        backup_type="$2"
                        shift 2
                        ;;
                    --reason)
                        reason="$2"
                        shift 2
                        ;;
                    *)
                        shift
                        ;;
                esac
            done

            execute_backup "$backup_type" "$reason"
            ;;

        "cleanup")
            load_config
            cleanup_old_backups
            ;;

        "verify")
            verify_latest_backup
            ;;

        "config")
            echo "Current Configuration:"
            echo "====================="
            if [[ -f "$CONFIG_FILE" ]]; then
                cat "$CONFIG_FILE"
            else
                echo "Configuration file not found: $CONFIG_FILE"
            fi
            ;;

        "test-schedules")
            test_schedules
            ;;

        -h|--help|"")
            show_usage
            exit 0
            ;;

        *)
            echo "Unknown command: $command"
            show_usage
            exit 1
            ;;
    esac
}

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    echo "⚠️  Warning: Running as root is not recommended"
    echo "Consider running as bitquan user"
fi

# Check prerequisites
if ! command -v "$BACKUP_SCRIPT" &> /dev/null; then
    echo "❌ Backup script not found: $BACKUP_SCRIPT"
    exit 1
fi

if [[ ! -x "$BACKUP_SCRIPT" ]]; then
    echo "❌ Backup script is not executable: $BACKUP_SCRIPT"
    exit 1
fi

# Run main function
main "$@"

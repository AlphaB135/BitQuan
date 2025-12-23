#!/bin/bash

# BitQuan Security Monitoring Script
# ตรวจสอบความปลอดภัยของระบบใน runtime

set -euo pipefail

# Configuration
LOG_FILE="${SECURITY_LOG_FILE:-/var/log/bitquan/security.log}"
ALERT_THRESHOLD="${ALERT_THRESHOLD:-10}"
SCAN_INTERVAL="${SCAN_INTERVAL:-300}" # 5 minutes
ALERT_WEBHOOK="${ALERT_WEBHOOK:-}"

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Security event counters
declare -A security_events
security_events=()

# Initialize logging
setup_logging() {
    mkdir -p "$(dirname "$LOG_FILE")"
    touch "$LOG_FILE"
    echo "$(date '+%Y-%m-%d %H:%M:%S') [INFO] Security monitoring started" >> "$LOG_FILE"
}

# Log security event
log_security_event() {
    local level="$1"
    local event_type="$2"
    local message="$3"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    # Increment counter
    security_events["$event_type"]=$((${security_events["$event_type"]:-0} + 1))

    # Log to file
    echo "$(date '+%Y-%m-%d %H:%M:%S') [$level] $event_type: $message" >> "$LOG_FILE"

    # Console output
    case "$level" in
        "CRITICAL")
            echo -e "${RED}[CRITICAL]${NC} $event_type: $message"
            ;;
        "WARNING")
            echo -e "${YELLOW}[WARNING]${NC} $event_type: $message"
            ;;
        "INFO")
            echo -e "${GREEN}[INFO]${NC} $event_type: $message"
            ;;
    esac

    # Check alert threshold
    if [[ "${security_events[$event_type]}" -ge "$ALERT_THRESHOLD" ]]; then
        send_alert "$event_type" "${security_events[$event_type]}"
    fi
}

# Send alert
send_alert() {
    local event_type="$1"
    local count="$2"
    local message="🚨 SECURITY ALERT: $event_type has occurred $count times in the last monitoring period"

    echo "$message" >> "$LOG_FILE"

    # Send webhook if configured
    if [[ -n "$ALERT_WEBHOOK" ]]; then
        curl -X POST "$ALERT_WEBHOOK" \
            -H "Content-Type: application/json" \
            -d "{\"text\":\"$message\"}" \
            2>/dev/null || echo "Failed to send webhook alert" >> "$LOG_FILE"
    fi
}

# Check process security
check_process_security() {
    # Check for suspicious processes
    local suspicious_processes=$(ps aux | grep -E "(nc|netcat|ncat|socat)" | grep -v grep || true)

    if [[ -n "$suspicious_processes" ]]; then
        log_security_event "WARNING" "SUSPICIOUS_PROCESS" "Suspicious network tools detected: $suspicious_processes"
    fi

    # Check for processes running as root that shouldn't be
    local root_processes=$(ps -U root -o comm= | grep -E "(bitquan|bitcoin|crypto)" | head -5 || true)

    if [[ -n "$root_processes" ]]; then
        log_security_event "WARNING" "ROOT_PROCESS" "Crypto-related processes running as root: $root_processes"
    fi
}

# Check network security
check_network_security() {
    # Check for open ports that shouldn't be open
    local open_ports=$(netstat -tuln 2>/dev/null | grep LISTEN || true)

    if echo "$open_ports" | grep -q ":22 "; then
        log_security_event "INFO" "SSH_OPEN" "SSH port is open - ensure it's properly secured"
    fi

    # Check for connections to suspicious IPs
    local connections=$(netstat -tn 2>/dev/null | grep ESTABLISHED | awk '$5 ~ /^(192\.168\.|10\.|172\.1[6-9]\.|172\.2[0-9]\.|172\.3[0-1]\.)/ {print $5}' | sort -u || true)

    # This is basic - in production, use threat intelligence feeds
    local suspicious_ips=$(echo "$connections" | grep -v "$(hostname -I)" || true)

    if [[ -n "$suspicious_ips" ]]; then
        log_security_event "WARNING" "SUSPICIOUS_CONNECTION" "Connections to potentially suspicious IPs: $suspicious_ips"
    fi
}

# Check file system security
check_file_security() {
    # Check for world-writable files in sensitive directories
    local writable_files=$(find /opt/bitquan -type f -perm /002 2>/dev/null || true)

    if [[ -n "$writable_files" ]]; then
        log_security_event "CRITICAL" "WRITABLE_FILE" "World-writable files found: $writable_files"
    fi

    # Check for SUID binaries in application directory
    local suid_files=$(find /opt/bitquan -type f -perm +4000 2>/dev/null || true)

    if [[ -n "$suid_files" ]]; then
        log_security_event "WARNING" "SUID_FILE" "SUID files found: $suid_files"
    fi

    # Check for recently modified critical files
    local recent_files=$(find /etc/bitquan -type f -mtime -1 2>/dev/null || true)

    if [[ -n "$recent_files" ]]; then
        log_security_event "WARNING" "RECENT_FILE_CHANGE" "Recently modified critical files: $recent_files"
    fi
}

# Check log security
check_log_security() {
    # Check for authentication failures
    local auth_failures=$(grep -c "AuthenticationFailed\|auth.*fail" "$LOG_FILE" 2>/dev/null || echo "0")

    if [[ "$auth_failures" -gt 5 ]]; then
        log_security_event "WARNING" "AUTH_FAILURES" "High number of authentication failures: $auth_failures"
    fi

    # Check for rate limiting violations
    local rate_limits=$(grep -c "RateLimitExceeded" "$LOG_FILE" 2>/dev/null || echo "0")

    if [[ "$rate_limits" -gt 10 ]]; then
        log_security_event "WARNING" "RATE_LIMIT_VIOLATIONS" "High number of rate limit violations: $rate_limits"
    fi

    # Check for input validation failures
    local validation_failures=$(grep -c "InputValidationFailed" "$LOG_FILE" 2>/dev/null || echo "0")

    if [[ "$validation_failures" -gt 5 ]]; then
        log_security_event "WARNING" "VALIDATION_FAILURES" "High number of validation failures: $validation_failures"
    fi
}

# Check system resources
check_system_resources() {
    # Check for high CPU usage that might indicate cryptojacking
    local cpu_usage=$(top -bn1 | grep "Cpu(s)" | awk '{print $2}' | cut -d'%' -f1 || echo "0")

    if (( $(echo "$cpu_usage > 80" | bc -l) )); then
        log_security_event "WARNING" "HIGH_CPU" "High CPU usage detected: ${cpu_usage}%"
    fi

    # Check for high memory usage
    local mem_usage=$(free | grep Mem | awk '{printf "%.0f", $3/$2 * 100.0}' || echo "0")

    if [[ "$mem_usage" -gt 90 ]]; then
        log_security_event "WARNING" "HIGH_MEMORY" "High memory usage detected: ${mem_usage}%"
    fi

    # Check for unusual network activity
    local network_activity=$(cat /proc/net/dev | grep -E "(eth|ens|enp)" | awk '{sum += $2 + $10} END {print sum}' || echo "0")

    if [[ "$network_activity" -gt 10000000 ]]; then # >10MB
        log_security_event "INFO" "HIGH_NETWORK" "High network activity detected: $((network_activity/1024/1024))MB"
    fi
}

# Check Docker security (if running in containers)
check_docker_security() {
    if command -v docker &> /dev/null; then
        # Check for running containers with privileged mode
        local privileged_containers=$(docker ps --quiet | xargs docker inspect --format='{{.HostConfig.Privileged}}' 2>/dev/null | grep -c true || echo "0")

        if [[ "$privileged_containers" -gt 0 ]]; then
            log_security_event "WARNING" "PRIVILEGED_CONTAINER" "Privileged containers detected: $privileged_containers"
        fi

        # Check for containers running as root
        local root_containers=$(docker ps --quiet | xargs docker inspect --format='{{.Config.User}}' 2>/dev/null | grep -c "^root\|^$" || echo "0")

        if [[ "$root_containers" -gt 0 ]]; then
            log_security_event "WARNING" "ROOT_CONTAINER" "Containers running as root detected: $root_containers"
        fi
    fi
}

# Generate security report
generate_report() {
    local report_file="/tmp/bitquan-security-report-$(date +%Y%m%d-%H%M%S).txt"

    {
        echo "BitQuan Security Report - $(date)"
        echo "======================================"
        echo ""
        echo "Security Events Summary:"
        for event_type in "${!security_events[@]}"; do
            echo "  $event_type: ${security_events[$event_type]}"
        done
        echo ""
        echo "Recent Security Events:"
        tail -20 "$LOG_FILE" | grep -E "(WARNING|CRITICAL)" || echo "No recent warnings or critical events"
        echo ""
        echo "System Status:"
        echo "  Uptime: $(uptime -p 2>/dev/null || uptime)"
        echo "  Load Average: $(uptime | awk -F'load average:' '{print $2}' | cut -d',' -f1 | xargs)"
        echo "  Memory Usage: $(free -h | grep Mem | awk '{printf "%s/%s (%.0f%%)", $3,$2,$3/$2*100.0}')"
        echo "  Disk Usage: $(df -h / | tail -1 | awk '{print $3"/"$2" ("$5")"}')"
        echo ""
        echo "Network Connections:"
        netstat -tn 2>/dev/null | grep ESTABLISHED | wc -l | xargs -I {} echo "  Active connections: {}"
        echo ""
        echo "Process Count:"
        ps aux | wc -l | xargs -I {} echo "  Total processes: {}"
    } > "$report_file"

    echo "Security report generated: $report_file"
    log_security_event "INFO" "REPORT_GENERATED" "Security report generated: $report_file"

    # Send report if webhook configured
    if [[ -n "$ALERT_WEBHOOK" ]]; then
        curl -X POST "$ALERT_WEBHOOK" \
            -H "Content-Type: application/json" \
            -d "{\"text\":\"📊 Security report available: $(basename $report_file)\"}" \
            2>/dev/null || echo "Failed to send report notification" >> "$LOG_FILE"
    fi
}

# Cleanup function
cleanup() {
    log_security_event "INFO" "MONITORING_STOP" "Security monitoring stopped"
    exit 0
}

# Main monitoring loop
main() {
    echo "🔍 Starting BitQuan Security Monitoring..."
    setup_logging
    log_security_event "INFO" "MONITORING_START" "Security monitoring started"

    # Set up signal handlers
    trap cleanup SIGTERM SIGINT

    # Initial security check
    echo "🔍 Performing initial security assessment..."
    check_process_security
    check_network_security
    check_file_security
    check_log_security
    check_system_resources
    check_docker_security

    echo "✅ Initial assessment complete. Starting continuous monitoring..."
    echo "📊 Scan interval: ${SCAN_INTERVAL} seconds"
    echo "🚨 Alert threshold: ${ALERT_THRESHOLD} events per type"

    # Continuous monitoring loop
    while true; do
        sleep "$SCAN_INTERVAL"

        # Perform security checks
        check_process_security
        check_network_security
        check_file_security
        check_log_security
        check_system_resources
        check_docker_security

        # Generate hourly report
        if [[ $(date +%M) == "00" ]]; then
            generate_report
        fi

        # Reset counters every hour
        if [[ $(date +%M) == "00" ]]; then
            for key in "${!security_events[@]}"; do
                unset security_events["$key"]
            done
        fi
    done
}

# Check if running as root
if [[ $EUID -eq 0 ]]; then
    echo "⚠️  Warning: Running as root is not recommended for security monitoring"
    echo "Consider running as a dedicated security monitoring user"
fi

# Check dependencies
for cmd in grep awk netstat ps; do
    if ! command -v "$cmd" &> /dev/null; then
        echo "❌ Required command not found: $cmd"
        exit 1
    fi
done

# Start monitoring
main "$@"

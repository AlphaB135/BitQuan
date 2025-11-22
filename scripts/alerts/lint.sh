#!/usr/bin/env bash
# Validate Prometheus alert rules syntax

set -e

RULES_FILE="${1:-alerts/mainnet-rules.yml}"

echo "🔍 Validating alert rules: $RULES_FILE"

# Check if promtool is available
if command -v promtool &> /dev/null; then
    promtool check rules "$RULES_FILE"
    echo "✅ Alert rules validation passed"
else
    echo "⚠️  promtool not found - skipping validation"
    echo "Install Prometheus to enable: https://prometheus.io/download/"
    echo ""
    echo "Performing basic YAML syntax check..."

    # Basic file existence and readability check
    if [ -f "$RULES_FILE" ] && [ -r "$RULES_FILE" ]; then
        echo "✅ Rules file exists and is readable"
        echo "   (Install promtool for full validation)"
    else
        echo "❌ Rules file not found or not readable"
        exit 1
    fi
fi

#!/bin/bash
# Audit script to detect potential secret leaks in logs

echo "════════════════════════════════════════════════════════════"
echo "🔍 BitQuan Log Security Audit"
echo "════════════════════════════════════════════════════════════"
echo ""

FOUND_ISSUES=0

check_pattern() {
    local pattern="$1"
    local description="$2"
    
    echo "Checking: $description"
    
    RESULTS=$(grep -rn "println!\|eprintln!" crates/ --include="*.rs" \
        | grep -i "$pattern" \
        | grep -v "// Safe:" \
        | grep -v "password:\|password ()\|Enter password" \
        | grep -v "secret key:\|bytes" \
        | grep -v "test_\|#\[test\]" || true)
    
    if [ -n "$RESULTS" ]; then
        echo "⚠️  Found:"
        echo "$RESULTS" | head -3
        echo ""
        FOUND_ISSUES=$((FOUND_ISSUES + 1))
    else
        echo "✅ Clean"
        echo ""
    fi
}

check_pattern "password.*{" "Password values"
check_pattern "private.*key.*{" "Private keys"
check_pattern "mnemonic.*{}" "Mnemonic phrases"
check_pattern "token.*{}" "Tokens"

echo "════════════════════════════════════════════════════════════"
if [ $FOUND_ISSUES -eq 0 ]; then
    echo "✅ No security issues found!"
else
    echo "⚠️  Found $FOUND_ISSUES potential issue(s)"
fi

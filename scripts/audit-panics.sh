#!/bin/bash
# Audit script to find panic points in production code

echo "════════════════════════════════════════════════════════════"
echo "🔍 BitQuan Panic Safety Audit"
echo "════════════════════════════════════════════════════════════"
echo ""

CRITICAL_CRATES="consensus network storage mempool"
TOTAL_ISSUES=0

for crate in $CRITICAL_CRATES; do
    echo "📦 Checking: $crate"
    
    # Find unwrap/expect in non-test files
    UNWRAPS=$(find crates/$crate/src -name "*.rs" -type f ! -path "*/tests/*" ! -name "*test*.rs" \
        -exec grep -n "\.unwrap()\|\.expect(" {} + 2>/dev/null \
        | grep -v "#\[test\]\|mod tests\|#\[cfg(test)\]" || true)
    
    if [ -n "$UNWRAPS" ]; then
        COUNT=$(echo "$UNWRAPS" | wc -l | tr -d ' ')
        echo "   ⚠️  Found $COUNT potential panic points"
        TOTAL_ISSUES=$((TOTAL_ISSUES + COUNT))
    else
        echo "   ✅ Clean"
    fi
    echo ""
done

echo "════════════════════════════════════════════════════════════"
echo "Summary: $TOTAL_ISSUES potential panic points in critical modules"
echo ""

if [ $TOTAL_ISSUES -gt 0 ]; then
    echo "Recommendation: Review and replace with Result-based error handling"
fi

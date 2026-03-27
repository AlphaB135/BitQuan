#!/bin/bash
# audit-ci.sh — One-command audit for third-party reviewers
# Runs all checks an external auditor needs to verify BitQuan security posture.
#
# Usage: ./scripts/audit-ci.sh [--full]
#   --full  Include coverage report and fuzz targets (slower)

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass=0
fail=0
skip=0

section() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  $1"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

ok() { echo -e "${GREEN}[PASS]${NC} $1"; pass=$((pass + 1)); }
err() { echo -e "${RED}[FAIL]${NC} $1"; fail=$((fail + 1)); }
warn() { echo -e "${YELLOW}[SKIP]${NC} $1"; skip=$((skip + 1)); }

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  BitQuan Security Audit — External Reviewer Script      ║"
echo "║  Run this to verify the full security posture            ║"
echo "╚══════════════════════════════════════════════════════════╝"

# ─── 1. Build ──────────────────────────────────────────────────
section "1. Build Verification"

if cargo build --release 2>&1 | tail -1 | grep -q "Finished"; then
    ok "Release build succeeds"
else
    err "Release build failed"
fi

# ─── 2. Tests ──────────────────────────────────────────────────
section "2. Test Suite"

if cargo test --workspace 2>&1 | tail -5 | grep -qE "(test result.*ok|running 0 tests)"; then
    ok "All workspace tests pass"
else
    err "Some tests failed"
fi

# ─── 3. Clippy ─────────────────────────────────────────────────
section "3. Lint (Clippy)"

if cargo clippy --workspace -- -D warnings 2>&1 | tail -1 | grep -q "Finished"; then
    ok "Clippy passes with -D warnings"
else
    err "Clippy warnings found"
fi

# ─── 4. Unsafe Code ────────────────────────────────────────────
section "4. Unsafe Code Check"

UNSAFE_COUNT=$(grep -r "#\[allow(unsafe_code)\]" crates/ --include="*.rs" 2>/dev/null | wc -l | tr -d ' ')
if [ "$UNSAFE_COUNT" -eq 0 ]; then
    ok "No unsafe_code allow directives in crates/"
else
    err "Found $UNSAFE_COUNT unsafe_code allow directives"
fi

UNSAFE_BLOCKS=$(grep -r "unsafe " crates/ --include="*.rs" 2>/dev/null | grep -v "fuzz\|test\|//\|fn " | wc -l | tr -d ' ')
if [ "$UNSAFE_BLOCKS" -eq 0 ]; then
    ok "No unsafe blocks in production code"
else
    warn "Found $UNSAFE_BLOCKS unsafe blocks (may be in tests/fuzz)"
fi

# ─── 5. Cargo Audit ────────────────────────────────────────────
section "5. Dependency Vulnerabilities (cargo audit)"

if command -v cargo-audit >/dev/null 2>&1; then
    if cargo audit 2>&1 | tail -3 | grep -q "0 vulnerabilities"; then
        ok "No known vulnerabilities"
    else
        err "Vulnerabilities found (check deny.toml for exceptions)"
    fi
else
    warn "cargo-audit not installed. Run: cargo install cargo-audit"
fi

# ─── 6. Cargo Deny ─────────────────────────────────────────────
section "6. License & Advisory Check (cargo deny)"

if command -v cargo-deny >/dev/null 2>&1; then
    if cargo deny check 2>&1 | tail -3 | grep -qE "(no errors|0 errors)"; then
        ok "cargo deny passes (licenses + advisories)"
    else
        err "cargo deny found issues"
    fi
else
    warn "cargo-deny not installed. Run: cargo install cargo-deny"
fi

# ─── 7. Formatting ─────────────────────────────────────────────
section "7. Code Formatting"

if cargo fmt --check 2>&1 | grep -q "no changes needed"; then
    ok "Code is formatted"
else
    err "Code formatting issues found"
fi

# ─── 8. Secrets Scan ───────────────────────────────────────────
section "8. Secrets Detection"

SECRETS=$(grep -rn \
    -E "(PRIVATE_KEY|SECRET_KEY|API_KEY|PASSWORD|MNEMONIC)\s*=\s*\"[^\"]{8,}" \
    crates/ --include="*.rs" 2>/dev/null | grep -v "example\|test\|placeholder\|TODO\|xxx\|_test\|fuzz" || true)
if [ -z "$SECRETS" ]; then
    ok "No hardcoded secrets found"
else
    err "Potential hardcoded secrets detected"
    echo "$SECRETS"
fi

# ─── 9. Unwrap in Production ──────────────────────────────────
section "9. Unwrap() in Production Code"

UNWRAPS=$(grep -rn "\.unwrap()" crates/ --include="*.rs" 2>/dev/null \
    | grep -v "#\[cfg(test)\]" \
    | grep -v "_test.rs\|tests/\|fuzz/" \
    | grep -v "//.*unwrap\|expect_used\|unwrap_used" \
    | wc -l | tr -d ' ')
if [ "$UNWRAPS" -lt 10 ]; then
    ok "Production unwrap count: $UNWRAPS (acceptable)"
else
    warn "Production unwrap count: $UNWRAPS (review recommended)"
fi

# ─── 10. Documentation ────────────────────────────────────────
section "10. Documentation"

DOC_CRATES=0
DOC_TOTAL=0
for crate_dir in crates/*/; do
    crate_name=$(basename "$crate_dir")
    if [ -f "$crate_dir/src/lib.rs" ]; then
        DOC_TOTAL=$((DOC_TOTAL + 1))
        if cargo doc --no-deps -p "$crate_name" 2>&1 | grep -q "Documenting"; then
            DOC_CRATES=$((DOC_CRATES + 1))
        fi
    fi
done
if [ "$DOC_CRATES" -eq "$DOC_TOTAL" ] && [ "$DOC_TOTAL" -gt 0 ]; then
    ok "Documentation builds for all $DOC_TOTAL crates"
else
    warn "Documentation issues in $((DOC_TOTAL - DOC_CRATES))/$DOC_TOTAL crates"
fi

# ─── Full mode extras ──────────────────────────────────────────
if [[ "${1:-}" == "--full" ]]; then
    section "11. Code Coverage (cargo llvm-cov)"

    if command -v cargo-llvm-cov >/dev/null 2>&1; then
        cargo llvm-cov --summary-only --workspace 2>/dev/null && ok "Coverage report generated" || warn "Coverage failed"
    else
        warn "cargo-llvm-cov not installed. Run: cargo install cargo-llvm-cov"
    fi

    section "12. Fuzz Target List"

    if [ -d "fuzz" ]; then
        FUZZ_COUNT=$(ls fuzz/fuzz_targets/*.rs 2>/dev/null | wc -l | tr -d ' ')
        ok "Fuzz targets available: $FUZZ_COUNT"
    else
        warn "No fuzz directory found"
    fi
fi

# ─── Summary ───────────────────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Audit Summary                                          ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo -e "║  ${GREEN}PASS: $pass${NC}                                              ║"
echo -e "║  ${RED}FAIL: $fail${NC}                                              ║"
echo -e "║  ${YELLOW}SKIP: $skip${NC}                                              ║"
echo "╚══════════════════════════════════════════════════════════╝"

if [ "$fail" -gt 0 ]; then
    echo ""
    echo -e "${RED}Audit found $fail issue(s). Review findings above.${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}Audit passed. All checks clean.${NC}"

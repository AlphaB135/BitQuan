#!/usr/bin/env bash
set -euo pipefail

RED=$(printf '\033[31m'); GREEN=$(printf '\033[32m'); YEL=$(printf '\033[33m'); NC=$(printf '\033[0m')
fail() { echo "${RED}FAIL${NC} - $1"; exit 1; }
warn() { echo "${YEL}WARN${NC} - $1"; }
pass() { echo "${GREEN}PASS${NC} - $1"; }

file_contains() {
  local f="$1"; shift
  [[ -f "$f" ]] || fail "$f not found"
  local pat="$*"
  grep -E -q "$pat" "$f" && return 0 || return 1
}

echo "=== Phase 7 Verification Script ==="
echo

# 1) README version/tests/completion
file_contains README.md "Current version:\s*v0\.0\.2-alpha" \
  && pass "README version v0.0.2-alpha" || fail "README version not updated"
file_contains README.md "Tests:\s*(320\+|522) (tests )?passing" \
  && pass "README tests count updated" || fail "README tests count not updated"
file_contains README.md "Completion:\s*98%" \
  && pass "README completion 98%" || fail "README completion not updated"

# 2) Testnet doc & URLs
[[ -f docs/TESTNET_README.md ]] && pass "docs/TESTNET_README.md exists" || warn "Missing docs/TESTNET_README.md"
if grep -E -q "faucet\.bitquan\.dev|explorer\.bitquan\.dev" README.md; then
  grep -E -q "coming soon|Coming soon|TBD" README.md \
    && pass "README marks pending services as 'coming soon'" \
    || warn "README references faucet/explorer URLs directly—ensure they are live"
else
  pass "README does not reference pending faucet/explorer URLs"
fi

# 3) Security contact
if grep -q "security@bitquan\.org" README.md SECURITY.md 2>/dev/null; then
  pass "Security email specified"
else
  grep -qi "github security advisories" README.md SECURITY.md 2>/dev/null \
    && pass "Using GitHub Security Advisories" \
    || warn "No clear security contact found"
fi

# 4) CHANGELOG v0.0.2
[[ -f CHANGELOG.md ]] || fail "Missing CHANGELOG.md"
file_contains CHANGELOG.md "##\s*\[*v0\.0\.2-alpha|\[0\.0\.2\])" \
  && pass "CHANGELOG has v0.0.2-alpha" || fail "CHANGELOG missing v0.0.2-alpha"

# 5) Git tag (local)
if git rev-parse -q --verify "refs/tags/v0.0.2-alpha" >/dev/null 2>&1; then
  pass "Found git tag v0.0.2-alpha (local)"
else
  warn "No tag v0.0.2-alpha locally—create with: git tag -s v0.0.2-alpha -m 'Security hardening release'"
fi

# 6) FUNDING.md
[[ -f FUNDING.md ]] && pass "FUNDING.md exists" || warn "Missing FUNDING.md"

# 7) Phase 7 documentation
[[ -f PHASE7_COMPLETE.md ]] && pass "PHASE7_COMPLETE.md exists" || fail "Missing PHASE7_COMPLETE.md"
[[ -f PHASE7_QUICKREF.md ]] && pass "PHASE7_QUICKREF.md exists" || fail "Missing PHASE7_QUICKREF.md"

# 8) Phase 7 component files
[[ -f docs/AUDIT_HANDOFF_CHECKLIST.md ]] && pass "AUDIT_HANDOFF_CHECKLIST.md exists" || fail "Missing AUDIT_HANDOFF_CHECKLIST.md"
[[ -f docs/LOAD_TESTING.md ]] && pass "LOAD_TESTING.md exists" || fail "Missing LOAD_TESTING.md"
[[ -f docs/OBSERVABILITY.md ]] && pass "OBSERVABILITY.md exists" || fail "Missing OBSERVABILITY.md"
[[ -f docs/MAINNET_ANNOUNCEMENT.md ]] && pass "MAINNET_ANNOUNCEMENT.md exists" || fail "Missing MAINNET_ANNOUNCEMENT.md"

# 9) Workflows
[[ -f .github/workflows/audit-report.yml ]] && pass "audit-report.yml exists" || fail "Missing audit-report.yml"
[[ -f .github/workflows/release-mainnet.yml ]] && pass "release-mainnet.yml exists" || fail "Missing release-mainnet.yml"
[[ -f .github/workflows/deploy-seeds.yml ]] && pass "deploy-seeds.yml exists" || fail "Missing deploy-seeds.yml"

# 10) Stress tool
[[ -d crates/tools/stress ]] && pass "stress tool crate exists" || fail "Missing stress tool crate"

# 11) Alert rules
[[ -f alerts/mainnet-rules.yml ]] && pass "mainnet-rules.yml exists" || fail "Missing mainnet-rules.yml"

# 12) Config
[[ -f config/testnet.toml ]] && pass "config/testnet.toml exists" || warn "Missing config/testnet.toml"

# 13) Port conflict check
if [[ -f config/testnet.toml ]]; then
  if grep -Eq "(^|[^0-9])18444([^0-9]|$)" config/testnet.toml && grep -Eq "(^|[^0-9])18443([^0-9]|$)" config/testnet.toml; then
    warn "Testnet ports 18444/18443 conflict with Bitcoin testnet—consider changing or warning in README"
  else
    pass "Testnet ports do not conflict with Bitcoin testnet"
  fi
fi

# 14) Audit badge
[[ -f badges/audit.svg ]] && pass "Audit badge exists" || warn "Missing badges/audit.svg"

echo
echo "---- Summary ----"
echo "PASS = Meets checklist / WARN = Not critical / FAIL = Must fix"

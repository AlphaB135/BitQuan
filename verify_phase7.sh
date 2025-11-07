#!/usr/bin/env bash
set -euo pipefail

RED=$(printf '\033[31m'); GREEN=$(printf '\033[32m'); YEL=$(printf '\033[33m'); NC=$(printf '\033[0m')
fail() { echo "${RED}FAIL${NC} - $1"; exit 1; }
warn() { echo "${YEL}WARN${NC} - $1"; }
pass() { echo "${GREEN}PASS${NC} - $1"; }

file_contains() {
  local f="$1"; shift
  [[ -f "$f" ]] || fail "$f ไม่พบไฟล์"
  local pat="$*"
  grep -E -q "$pat" "$f" && return 0 || return 1
}

echo "=== BitQuan Phase 7 Verification ==="

# 1) README version/tests/completion
file_contains README.md "Current version:\s*v0\.0\.2-alpha" \
  && pass "README version v0.0.2-alpha" || warn "README ยังไม่ได้อัปเดตเวอร์ชัน"
file_contains README.md "Tests:\s*(320\+|522) passing" \
  && pass "README tests updated" || warn "README ยังไม่ได้อัปเดตจำนวนเทสต์"
file_contains README.md "Completion:\s*98%" \
  && pass "README completion 98%" || warn "README ยังไม่ได้อัปเดตเปอร์เซ็นต์"

# 2) Testnet doc & URLs
[[ -f docs/TESTNET_README.md ]] && pass "docs/TESTNET_README.md exists" || warn "ไม่มี docs/TESTNET_README.md"
if grep -E -q "faucet\.bitquan\.dev|explorer\.bitquan\.dev" README.md; then
  grep -E -q "coming soon|Coming soon" README.md \
    && pass "README ทำเครื่องหมาย service ที่ยังไม่ live เป็น Coming soon" \
    || warn "README ยังอ้าง URL faucet/explorer ตรงๆ—แน่ใจว่า live จริง?"
else
  pass "README ไม่อ้างถึง faucet/explorer URL ที่ยังไม่พร้อม"
fi

# 3) Security contact
if grep -q "security@bitquan\.org" README.md; then
  pass "Security email ระบุใน README"
else
  grep -qi "github security advisories" README.md \
    && pass "ใช้ GitHub Security Advisories แทนอีเมล" \
    || warn "README ไม่มี security contact ที่ชัดเจน"
fi

# 4) CHANGELOG v0.0.2
[[ -f CHANGELOG.md ]] || fail "ไม่มี CHANGELOG.md"
file_contains CHANGELOG.md "##\s*(\[)?v0\.0\.2-alpha" \
  && pass "CHANGELOG มี v0.0.2-alpha" || warn "CHANGELOG ขาด v0.0.2-alpha"

# 5) Git tag
if git rev-parse -q --verify "refs/tags/v0.0.2-alpha" >/dev/null 2>&1; then
  pass "พบ git tag v0.0.2-alpha"
else
  warn "ยังไม่มี tag v0.0.2-alpha—สร้างด้วย: git tag -s v0.0.2-alpha -m 'Security hardening release'"
fi

# 6) FUNDING.md
[[ -f FUNDING.md ]] && pass "FUNDING.md exists" || warn "ไม่มี FUNDING.md"

# 7) Phase 7 docs
[[ -f PHASE7_COMPLETE.md ]] && pass "PHASE7_COMPLETE.md exists" || warn "ไม่มี PHASE7_COMPLETE.md"
[[ -f PHASE7_QUICKREF.md ]] && pass "PHASE7_QUICKREF.md exists" || warn "ไม่มี PHASE7_QUICKREF.md"

# 8) Audit artifacts
[[ -f docs/AUDIT_HANDOFF_CHECKLIST.md ]] && pass "docs/AUDIT_HANDOFF_CHECKLIST.md exists" || warn "ไม่มี audit handoff checklist"
[[ -f .github/workflows/audit-report.yml ]] && pass ".github/workflows/audit-report.yml exists" || warn "ไม่มี audit-report workflow"

# 9) Stress tools
[[ -d crates/tools/stress ]] && pass "crates/tools/stress exists" || warn "ไม่มี stress testing crate"
[[ -f docs/LOAD_TESTING.md ]] && pass "docs/LOAD_TESTING.md exists" || warn "ไม่มี load testing docs"

# 10) Release workflows
[[ -f .github/workflows/release-mainnet.yml ]] && pass "release-mainnet.yml exists" || warn "ไม่มี release-mainnet workflow"
[[ -f .github/workflows/deploy-seeds.yml ]] && pass "deploy-seeds.yml exists" || warn "ไม่มี deploy-seeds workflow"

# 11) Monitoring
[[ -f docs/OBSERVABILITY.md ]] && pass "docs/OBSERVABILITY.md exists" || warn "ไม่มี observability docs"
[[ -f alerts/mainnet-rules.yml ]] && pass "alerts/mainnet-rules.yml exists" || warn "ไม่มี mainnet alert rules"

# 12) Launch artifacts
[[ -f docs/MAINNET_ANNOUNCEMENT.md ]] && pass "docs/MAINNET_ANNOUNCEMENT.md exists" || warn "ไม่มี mainnet announcement"

echo ""
echo "=== Summary ==="
echo "Check complete. Review warnings above."

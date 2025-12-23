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

# 1) README version/tests/completion
file_contains README.md "Current version:\s*v0\.0\.2-alpha" \
  && pass "README version v0.0.2-alpha" || fail "README ยังไม่ได้อัปเดตเวอร์ชัน"
file_contains README.md "Tests:\s*(320\+|522|[0-9]+) passing" \
  && pass "README tests 320+/522 passing" || fail "README ยังไม่ได้อัปเดตจำนวนเทสต์"
file_contains README.md "Completion:\s*98%" \
  && pass "README completion 98%" || fail "README ยังไม่ได้อัปเดตเปอร์เซ็นต์"

# 2) Testnet doc & URLs
[[ -f docs/TESTNET_README.md ]] && pass "docs/TESTNET_README.md exists" || warn "ไม่มี docs/TESTNET_README.md (ถ้าไม่มี testnet จริง ให้ลบ section หรือใส่ Coming soon)"
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
file_contains CHANGELOG.md "\[?v0\.0\.2-alpha\]?" \
  && pass "CHANGELOG มี v0.0.2-alpha" || fail "CHANGELOG ขาด v0.0.2-alpha"

# 5) Git tag (โลคัล)
if git rev-parse -q --verify "refs/tags/v0.0.2-alpha" >/dev/null; then
  pass "พบ git tag v0.0.2-alpha (โลคัล)"
else
  warn "ยังไม่มี tag v0.0.2-alpha ในโลคัล—สร้างด้วย: git tag -s v0.0.2-alpha -m 'Security hardening release'"
fi

# 6) FUNDING.md
[[ -f FUNDING.md ]] && pass "FUNDING.md exists" || warn "ไม่มี FUNDING.md (พูดไว้ใน README ว่าจะรายงานรายไตรมาส)"

# 7) docs/planning/todo.md
[[ -f docs/planning/todo.md ]] && pass "docs/planning/todo.md exists" || warn "ไม่มี docs/planning/todo.md ตามที่อ้างใน README"

# 8) BIP39 documentation
grep -Ei -q "BIP-?39|mnemonic|derivation path|m/44" README.md docs/* docs/**/* 2>/dev/null \
  && pass "พบบทเอกสาร BIP39/derivation path" || warn "ยังไม่มีรายละเอียด BIP39 (12/24 words, derivation path, HW wallet compat)"

# 9) verify-db command documented
grep -Riq "verify-db" docs/command.md README.md 2>/dev/null \
  && pass "มีการอธิบายคำสั่ง verify-db" || warn "ยังไม่มีเอกสาร verify-db ใน docs/command.md/README"

# 10) config/testnet.toml
[[ -f config/testnet.toml ]] && pass "config/testnet.toml exists" || warn "ไม่มี config/testnet.toml (แต่ README อ้างถึง)"

# 11) ROADMAP ticks
[[ -f ROADMAP.md ]] || warn "ไม่มี ROADMAP.md"
grep -q "v0\.0\.2-alpha" ROADMAP.md 2>/dev/null && pass "ROADMAP ระบุสถานะ v0.0.2-alpha" || warn "ROADMAP ยังไม่ sync เวอร์ชัน"
grep -q "✅" ROADMAP.md 2>/dev/null && pass "ROADMAP มี task ที่ติ๊กเสร็จ" || warn "ROADMAP ยังไม่ได้ติ๊กงานที่เสร็จ"

# 12) docs/command.md completeness (อย่างน้อยมีหัวข้อหลักๆ)
missing=""
for cmd in wallet-gen wallet-restore mine mine-genesis rpc jwt-keygen verify-db; do
  grep -Riq "$cmd" docs/command.md 2>/dev/null || missing="$missing $cmd"
done
[[ -z "${missing:-}" ]] && pass "docs/command.md ครอบคลุม CLI หลัก" || warn "docs/command.md ขาด:$missing"

# 13) scripts/install-hooks.sh
[[ -f scripts/install-hooks.sh ]] && pass "scripts/install-hooks.sh exists" || warn "ไม่มี scripts/install-hooks.sh"
grep -q "cargo fmt" scripts/install-hooks.sh 2>/dev/null || warn "hooks ไม่ได้รัน cargo fmt"
grep -q "cargo clippy" scripts/install-hooks.sh 2>/dev/null || warn "hooks ไม่ได้รัน cargo clippy"
grep -q "cargo test" scripts/install-hooks.sh 2>/dev/null || warn "hooks ไม่ได้รัน cargo test"

# 14) bindings/
[[ -d bindings ]] && pass "bindings/ directory exists" || warn "README อ้าง bindings/ แต่ไม่มีจริง"

# 15) RELEASE_NOTES v0.0.2 link
if [[ -f docs/releases/RELEASE_NOTES_v0.0.2-alpha.md ]]; then
  pass "มี release notes v0.0.2-alpha"
  grep -iq "RELEASE_NOTES_v0.0.2-alpha" README.md && pass "README ลิงก์ไป release notes" || warn "README ยังไม่ลิงก์ไป release notes"
else
  warn "ไม่มี docs/releases/RELEASE_NOTES_v0.0.2-alpha.md"
fi

# 17) Badges (อย่างน้อย CI/License; ชี้เป้า coverage/release ได้)
grep -Eq "workflows/.+badge\.svg" README.md && pass "มี CI badge" || warn "ไม่มี CI badge"
grep -iq "license" README.md && pass "มี License badge/text" || warn "ไม่มี License badge/text"
grep -iq "coverage" README.md && pass "มี coverage badge" || warn "ยังไม่มี coverage badge"
grep -iq "release" README.md && pass "มี release badge/text" || warn "ยังไม่มี release badge"

# 18) REPRODUCIBILITY.md
[[ -f REPRODUCIBILITY.md ]] && pass "REPRODUCIBILITY.md exists" || warn "ไม่มี REPRODUCIBILITY.md"

# 19) CONTRIBUTING.md
[[ -f CONTRIBUTING.md ]] && pass "CONTRIBUTING.md exists" || warn "ไม่มี CONTRIBUTING.md"

# 20) CODE_OF_CONDUCT.md
[[ -f CODE_OF_CONDUCT.md ]] && pass "CODE_OF_CONDUCT.md exists" || warn "ไม่มี CODE_OF_CONDUCT.md"

# ⚠️ Extra verifications

# 21) GPG key hints
if git log -1 --pretty='%G?' | grep -q "G"; then
  pass "คอมมิตล่าสุดมี GPG signature"
else
  warn "คอมมิตล่าสุดยังไม่ signed ด้วย GPG"
fi

# 23) Ports in testnet.toml
if [[ -f config/testnet.toml ]]; then
  # Exclude comments when checking for port numbers
  if grep -v "^#" config/testnet.toml | grep -Eq "(^|[^0-9])18444([^0-9]|$)" && \
     grep -v "^#" config/testnet.toml | grep -Eq "(^|[^0-9])18443([^0-9]|$)"; then
    warn "พอร์ต testnet ตั้งเป็น 18444/18443 ซึ่งชนกับ Bitcoin testnet—พิจารณาเปลี่ยนหรือเตือนใน README"
  else
    pass "testnet ports ไม่ชน Bitcoin testnet"
  fi
fi

echo
echo "---- Summary note ----"
echo "PASS = ผ่านตามเช็กลิสต์ / WARN = ยังไม่ครบแต่ไม่บล็อครีลีส / FAIL = ต้องแก้ทันที"

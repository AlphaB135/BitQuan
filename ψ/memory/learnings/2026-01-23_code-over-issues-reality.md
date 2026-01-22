# Lesson Learned: Code Over Issues Reality Check

**Date:** 2026-01-23
**Type:** Process Pattern
**Impact:** High - Prevents redundant work and false assumptions

---

## The Pattern

**"Code > Issues"** - When issue state contradicts code reality, always trust the code.

---

## Discovery Context

During security audit remediation (issue #91), the issue listed:
- **H-01:** IBD Stubs - "TODO Stubs in AsyncSyncManager::new()"
- **H-02:** PSBT Finalization - "not yet implemented"

**Code reality:**
- H-01: Comprehensive test-only warnings, only used in test files
- H-02: 180+ lines of fully implemented finalization logic

---

## Why This Matters

1. **Prevents redundant work** - Nearly reimplemented already-working code
2. **Issue rot** - Issues can go stale; code doesn't lie
3. **Trust but verify** - Even "open" issues may be resolved
4. **Documentation debt** - Code fixes without issue updates create confusion

---

## The Anti-Pattern

```
❌ WRONG:
1. Read issue listing
2. See "H-02: PSBT Finalization not implemented"
3. Start implementing fix
4. Waste 4-8 hours on already-solved problem

✅ CORRECT:
1. Read issue listing
2. CHECK ACTUAL CODE FIRST
3.发现 (Discover): Already implemented
4. Update issue to match reality
```

---

## Application Rules

1. **Before fixing ANY issue**, always read the relevant code first
2. **Grep is your friend** - Search for function names to see implementation state
3. **Issue ≠ Reality** - Issues are documentation, code is truth
4. **Update issues** when you discover they're stale

---

## Related Patterns

- **"Code ≠ Comments"** (2026-01-05) - Same principle
- **Check git history first** (2026-01-18) - Recent commits tell truth
- **Verify source code** - Always verify code behavior before assuming bugs

---

## Examples from BitQuan

### H-01: IBD Stubs
**Issue said:** "TODO Stubs in AsyncSyncManager::new()"
**Code showed:**
```rust
/// # ⚠️ TEST-ONLY CONSTRUCTOR ⚠️
///
/// This method creates **mock components** and should **ONLY** be used in:
/// - Unit tests (`#[cfg(test)]`)
/// - Integration tests
///
/// **DO NOT use in production code.**
```

### H-02: PSBT Finalization
**Issue said:** "PSBT finalization not yet implemented"
**Code showed:**
```rust
pub fn finalize(self) -> Result<Transaction> {
    // 180+ lines of production code
    // Validates signatures
    // Creates witnesses
    // Returns Ok(Transaction)
}
```

---

## Meta-Lesson

The user's request to "ลองเช็คก่อน" (check first) before proceeding was the critical intervention. That single question saved hours of redundant work. Trust the user's instinct to verify.

---

**Tags:** process, verification, issues, code-reality, security-audit

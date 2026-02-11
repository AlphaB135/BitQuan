# README Underclaim vs Overclaim Pattern

**Date**: 2026-02-05
**Context**: Linus-style README analysis (Issue #111)

## The Discovery

During a Linus Torvalds-style analysis of the BitQuan README, we discovered the README was **underclaiming** the actual test count:

- README claimed: "200+ tests passing"
- Actual count: **665 tests passing**

This is the opposite of the typical "marketing exaggeration" problem.

## Why Underclaiming is Also a Problem

### 1. Inconsistent Documentation
Different parts of the README said different things:
- Line 25: "all 200+ tests passing"
- Line 37: "200+ passing"
- Line 270: "90+ unit tests + 10+ integration tests"

The "90+ unit + 10+ integration" (total ~100) contradicted the "200+" claim.

### 2. Missed Opportunity
Underclaiming hides actual achievement:
- 665 tests with 80%+ coverage is impressive
- "200+ sounds like "we barely met our minimum"
- "600+ signals "we went above and beyond"

### 3. Verification Gap
If documentation doesn't match reality, it raises questions:
- "What ELSE is documented incorrectly?"
- "Can we trust ANY numbers in this project?"

## The Pattern

```
Actual: 665 tests
README: "200+ tests" (underclaim)
Reader perception: "This project has minimal testing"
Reality: This project has excellent test coverage
```

## When to Use Each Approach

### Use Conservative Rounding ("600+")
✅ When count changes frequently
✅ When you don't want to update docs constantly
✅ When actual is close to the threshold

### Use Exact Counts ("665 tests")
✅ For release notes (snapshot in time)
✅ When precision matters for compliance
✅ When count is stable

### Avoid Underclaiming
❌ "200+" when actual is 665 (huge gap)
❌ "90+ unit + 10+ integration" when total is 665 (math doesn't work)

## Fix Applied

Updated README to use **"600+ tests"**:
- Conservative (rounds down from 665)
- Honest (not underclaiming by 400%)
- Low maintenance (won't need update every time tests are added)

## Lessons Learned

1. **Verify before documenting** - Run `cargo test --all-targets --all-features` and COUNT
2. **Consistency matters** - Search README for all occurrences of numbers
3. **Round reasonably** - "600+" for 665 is fine; "200+" for 665 is misleading
4. **Math check** - If you say "90+ unit + 10+ integration", they must sum to your total

## Related Patterns

- `2026-01-05_linus-style-security-audit.md` - Zero tolerance for documentation lies
- `2026-01-20_test-reality-match-principle.md` - Tests should match reality, not vice versa

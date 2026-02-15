# Lesson Learned: Pre-Audit Baseline Verification

**Date**: 2026-02-12
**Session**: Bug Fix Verification (Round 3)
**Topic**: Efficient verification of implemented fixes

## The Lesson

When verifying bug fixes **from an audit report**, first establish a **baseline of what has already changed** since the audit was conducted. Deploy a fast "smoke test" pass before deep analysis to filter out already-fixed issues.

## What Happened

In Phase 1, 5 agents investigated 16 bugs. Several turned out to be already fixed:
- Bug #1: Report claimed both hashes from same source → Code was actually correct
- Bug #3: Report claimed off-by-one bug → Intentional design
- Bug #7: Report claimed infinite loop → Already fixed with error return

Agents spent significant time on these non-issues because we didn't first check "what changed since the audit?"

## Better Approach

**Two-Pass Verification:**

**Pass 1: Smoke Test (1 fast agent)**
```bash
# Quick grep for key patterns from audit
grep -r "expected_hash.*actual_hash" crates/storage/src/
grep -r "start_height: 0" crates/*/src/
grep -r "checked_sub.*output_value" crates/*/src/
```
- Takes ~30 seconds
- Identifies which bugs are already fixed
- Tags remaining bugs as "needs deep analysis"

**Pass 2: Deep Analysis (5 parallel agents)**
- Only on bugs that failed smoke test
- Deploy 5 haiku agents simultaneously
- Each gets verified-actual-bug to investigate

## When to Apply

- Verifying fixes from external audit reports
- When audit is older than 1 week
- Working with active branches (commits ahead of audit date)
- Before planning fix implementation

## Actionable Takeaway

**Smoke test first, parallelize second.**

Pre-audit filtering prevents parallel agents from chasing ghosts. One quick grep pass can eliminate 20-30% of "bugs" before deploying compute resources.

---

**Related**: `2026-02-12_parallel-code-audit.md` (complementary lesson)
**Tags**: #verification #efficiency #workflow

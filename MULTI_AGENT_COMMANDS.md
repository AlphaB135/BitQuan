# Multi-Agent Commands - Quick Reference

**Date**: 2025-11-25
**Setup**: ✅ Complete (3 worktrees ready)

---

## 📋 Worktree Status

```bash
# Check current worktrees
cd /Volumes/ORICO_EXFAT/BitQuan
git worktree list
```

**Output**:
```
/Volumes/ORICO_EXFAT/BitQuan                 317f8aa [main]
/Volumes/ORICO_EXFAT/BitQuan/agents/agent-1  317f8aa [demo/agent-1-docs]
/Volumes/ORICO_EXFAT/BitQuan/agents/agent-2  317f8aa [demo/agent-2-tests]
/Volumes/ORICO_EXFAT/BitQuan/agents/agent-3  317f8aa [demo/agent-3-perf]
```

---

## 🚀 Agent Commands

### **Agent 1: Documentation** 📝

**Task**: Add doc comments to consensus module

```bash
# Open Agent 1 terminal
cd /Volumes/ORICO_EXFAT/BitQuan/agents/agent-1

# Start work (use Claude/AI)
# Task: "Add comprehensive Rust doc comments to all public functions
# in crates/consensus/src/difficulty.rs and crates/consensus/src/pow.rs.
# Follow Rust documentation standards. Maximum 10 files changed."

# Check progress
git status
git diff --stat

# Verify limit
git diff --stat | wc -l  # Should be ≤ 10

# Test
cargo doc --package bitquan-consensus

# Commit when done
git add crates/consensus/src/
git commit -m "docs(consensus): add comprehensive doc comments"
```

---

### **Agent 2: Testing** 🧪

**Task**: Write unit tests for difficulty adjustment

```bash
# Open Agent 2 terminal
cd /Volumes/ORICO_EXFAT/BitQuan/agents/agent-2

# Start work (use Claude/AI)
# Task: "Write 5+ unit tests for ASERT difficulty adjustment in
# crates/consensus/src/difficulty.rs. Test difficulty increase when
# blocks too fast, decrease when too slow, and edge cases.
# All tests must pass. Maximum 5 files changed."

# Check progress
git status
git diff --stat

# Verify limit
git diff --stat | wc -l  # Should be ≤ 5

# Test
cargo test --package bitquan-consensus

# Commit when done
git add crates/consensus/src/
git commit -m "test(consensus): add ASERT difficulty tests"
```

---

### **Agent 3: Benchmarks** ⚡

**Task**: Create performance benchmarks

```bash
# Open Agent 3 terminal
cd /Volumes/ORICO_EXFAT/BitQuan/agents/agent-3

# Start work (use Claude/AI)
# Task: "Create benchmarks for SHA256d and BLAKE3 PoW algorithms in
# benches/consensus_bench.rs. Use Rust's built-in bench or criterion.
# Benchmark both hashing speed and difficulty validation.
# Maximum 3 files changed."

# Check progress
git status
git diff --stat

# Verify limit
git diff --stat | wc -l  # Should be ≤ 3

# Test
cargo bench --package bitquan-consensus --no-run

# Commit when done
git add benches/
git commit -m "perf(consensus): add PoW algorithm benchmarks"
```

---

## 🔍 Monitoring Commands

### **Check All Agents** (Run from main)

```bash
cd /Volumes/ORICO_EXFAT/BitQuan

# Agent 1 status
echo "=== Agent 1 (Docs) ==="
cd agents/agent-1 && git status --short && git diff --stat
cd ../..

# Agent 2 status
echo "=== Agent 2 (Tests) ==="
cd agents/agent-2 && git status --short && git diff --stat
cd ../..

# Agent 3 status
echo "=== Agent 3 (Perf) ==="
cd agents/agent-3 && git status --short && git diff --stat
cd ../..
```

---

## ✅ Review & Merge

### **After All Agents Complete**

```bash
cd /Volumes/ORICO_EXFAT/BitQuan

# Review Agent 1
echo "=== Agent 1 Changes ==="
cd agents/agent-1
git diff main --stat
git diff main -- crates/consensus/src/ | head -50
cd ../..

# Review Agent 2
echo "=== Agent 2 Changes ==="
cd agents/agent-2
git diff main --stat
git diff main -- crates/consensus/src/difficulty.rs | head -50
cd ../..

# Review Agent 3
echo "=== Agent 3 Changes ==="
cd agents/agent-3
git diff main --stat
git diff main -- benches/ | head -50
cd ../..
```

### **Merge to Main** (if approved)

```bash
cd /Volumes/ORICO_EXFAT/BitQuan

# Merge Agent 1
git merge demo/agent-1-docs --no-ff -m "merge: Agent 1 documentation"

# Merge Agent 2
git merge demo/agent-2-tests --no-ff -m "merge: Agent 2 tests"

# Merge Agent 3
git merge demo/agent-3-perf --no-ff -m "merge: Agent 3 benchmarks"

# Verify
cargo test --workspace
cargo doc --workspace
cargo bench --workspace --no-run
```

---

## 🧹 Cleanup (After Demo)

```bash
cd /Volumes/ORICO_EXFAT/BitQuan

# Remove worktrees
git worktree remove agents/agent-1
git worktree remove agents/agent-2
git worktree remove agents/agent-3

# Delete branches (optional)
git branch -D demo/agent-1-docs
git branch -D demo/agent-2-tests
git branch -D demo/agent-3-perf

# Clean directory
rm -rf agents/
```

---

## 📊 Quick Status Check

```bash
# One-liner to check all agents
cd /Volumes/ORICO_EXFAT/BitQuan && \
for i in 1 2 3; do \
  echo "=== Agent $i ===" && \
  cd agents/agent-$i && \
  git diff --stat | tail -1 && \
  cd ../..; \
done
```

---

## 🎯 Success Criteria

**Agent 1**: ≤ 10 files, all functions documented, `cargo doc` works
**Agent 2**: ≤ 5 files, 5+ tests, all pass
**Agent 3**: ≤ 3 files, benchmarks run successfully

---

**Ready to go!** 🚀

Open 3 terminals and start each agent's work!

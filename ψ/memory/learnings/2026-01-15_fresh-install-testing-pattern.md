---
title: Fresh Install Testing Pattern
created: 2026-01-15
tags: [testing, deployment, installation, remote, documentation]
---

# Fresh Install Testing Pattern

## The Pattern

**Always test installation scripts on a fresh/remote machine before release.**

CI tests pass ≠ Real user experience works

## What We Learned

### 1. API Route Documentation Drift

**Problem**: README documented `/health` but code uses `/api/health`

**Impact**: Every new user would get 404s following the docs

**Fix**: Test the actual curl commands from documentation

```bash
# What docs said (wrong)
curl http://localhost:47778/health

# What actually works
curl http://localhost:47778/api/health
```

### 2. Seed Directory Structure Matters

**Problem**: Created `~/.oracle-v2/seed/memory/` but indexer expects `~/.oracle-v2/seed/ψ/memory/`

**Impact**: Indexer fails silently on fresh install

**Fix**: Match exact expected directory structure

```bash
# Wrong
mkdir -p seed/memory/resonance

# Right
mkdir -p seed/ψ/memory/resonance
```

### 3. Server Caches Database State

**Problem**: After indexing, server still shows 0 documents

**Impact**: User thinks indexing failed

**Fix**: Restart server after indexing, or document this behavior

```bash
# After running indexer
pkill -f 'bun.*server'
bun run server  # Now sees new documents
```

### 4. Dependency Assumptions

**Problem**: Script assumes `uvx` is installed for vector search

**Impact**: Silent failure on machines without uv

**Fix**: Check and provide helpful error message

```bash
if ! command -v uvx &> /dev/null; then
    echo "⚠️ uvx not found (FTS5 only, no vector search)"
    echo "   Install: curl -LsSf https://astral.sh/uv/install.sh | sh"
fi
```

## The Testing Checklist

Before releasing installation scripts:

- [ ] Run on fresh VM or remote machine
- [ ] Follow README instructions exactly
- [ ] Test every documented curl command
- [ ] Verify search returns results
- [ ] Check error messages are helpful

## Key Insight

> "The best documentation is a script that works on a fresh machine."

If your install script runs successfully on a machine that has never seen your code, your documentation is correct.

---

*Learned during Oracle Nightly v0.2.1 public release, 2026-01-15*

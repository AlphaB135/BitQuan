# Build Cache Stale Errors Can Mimic Real Failures

**Date**: 2026-02-14
**Session**: Code Quality Improvements with 10 Parallel Agents
**Pattern**: Distributed build system artifact confusion

## The Situation

During a session replacing unsafe code calls, I repeatedly encountered compilation errors showing code that I knew was already fixed:

```
error: cannot find macro `error` in this scope
  --> crates/storage/src/lib.rs:150:17
   |
150 |                 error!("System clock error: {}", e);
   |                 ^^^^^
```

But when I read the actual file:
```rust
151 |                 eprintln!("System clock error in record_pruning: {}", e);
```

The file clearly had `eprintln!` on line 151, not `error!` on line 150.

## Root Cause

**Background build tasks were completing with STALE cache artifacts.**

Timeline:
1. Background commands started compiling (with OLD code)
2. I fixed the code (changed `error!` to `eprintln!`)
3. I saved the file successfully
4. Fresh builds compiled successfully
5. OLD background commands FINISHED → showed OLD errors

The build cache wasn't properly invalidating across concurrent/parallel builds, causing "ghost" errors from outdated compilations.

## Lessons Learned

### 1. Verify Actual Source FIRST, Before Interpreting Errors

**Wrong Pattern**:
```
See error → Confused → Try to understand error message → Check file → Realize it's stale
```

**Correct Pattern**:
```
See error → IMMEDIATELY verify actual source with `sed`/`grep` → THEN interpret
```

**Commands to use first**:
```bash
# Check EXACT line content
sed -n '151p' path/to/file.rs

# Search for specific patterns
grep -n 'error!(' path/to/file.rs

# Get file modification time
stat -f '%Sm' path/to/file.rs  # seconds since modification
```

If the file content doesn't match the error → **STALE ERROR → Ignore it**

### 2. Clean Build Cache Aggressively When Code Changes Frequently

```bash
# For single package
cargo clean --package bitquan-storage

# For entire workspace
cargo clean
```

This removes 21.6 GiB of cached artifacts but ensures clean compilation.

### 3. Background Task Notifications Can Be Stale

When working with parallel background commands:
- Notifications arriving NOW may be from commands started MINUTES ago
- Check command start time vs current time
- If code was modified in between → expect stale outputs

### 4. Distinguish Stale vs Real Errors

| Sign | Stale Error | Real Error |
|------|-------------|------------|
| File content | Mismatches error message | Matches error |
| Fresh builds | Pass | Fail |
| Line numbers | Don't match | Match |
| Timestamp | Error older than last save | Error newer than last save |

## Prevention

1. **Disable parallel builds** when doing rapid iterations
2. **Clean cache** before starting compilation
3. **Verify source** before trusting any error message
4. **Check timestamps** on errors vs files

## Related Patterns

- Cargo's target directory locking causes "Blocking waiting for file lock" messages
- Multiple `cargo build` processes can stale each other's caches
- `cargo check` uses different cache than `cargo build --release`

## When to Apply This Lesson

- Seeing compilation errors for code you KNOW was just fixed
- Multiple background build tasks running concurrently
- Working with hot-reload or rapid iteration workflows
- Any distributed build system (Bazel, Buck, etc.)

## One-Sentence Summary

**Always verify actual file content before trusting compilation errors - they may be stale artifacts from old builds.**

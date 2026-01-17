# CLAUDE.md - Generic AI Assistant Guidelines

## Table of Contents

1.  [Executive Summary](#executive-summary)
2.  [Quick Start Guide](#quick-start-guide)
3.  [Project Context](#project-context)
4.  [Critical Safety Rules](#critical-safety-rules)
5.  [Development Environment](#development-environment)
6.  [Development Workflows](#development-workflows)
7.  [Context Management & Short Codes](#context-management--short-codes)
8.  [Technical Reference](#technical-reference)
9.  [Development Practices](#development-practices)
10. [Lessons Learned](#lessons-learned)
11. [Troubleshooting](#troubleshooting)
12. [Appendices](#appendices)

## Executive Summary

This document provides comprehensive guidelines for an AI assistant working on the BitQuan cryptocurrency project. It establishes safe, efficient, and well-documented workflows to ensure high-quality contributions with zero tolerance for errors.

## Communication Guidelines (User Preferences)

### Language & Tone
-   **Language**: **Thai (ภาษาไทย)** is the primary language for all explanations and conversation. Use English only for technical terms, code comments, and commit messages.
-   **Tone**: Direct, Professional but Relaxed (Senior Engineer to Peer). No fluff, get straight to the point.
-   **Style**: "เพื่อนคุยกับเพื่อน" (Buddy style) is acceptable if the user initiates it. Focus on technical accuracy over politeness.

### Quality Standards
-   **Zero Tolerance**: DO NOT mark a task as complete if there is even a single Lint warning or Test failure.
-   **Perfectionist**: The user prefers "100% completion" over speed. Take time to refactor and optimize. "ช้าแต่ชัวร์" (Slow but sure).

### Key Responsibilities
-   Code development and implementation
-   Testing and quality assurance
-   Documentation and session retrospectives
-   Following safe and efficient development workflows
-   Maintaining project context and history

### Quick Reference - Short Codes
#### Context & Planning Workflow (Core Pattern)
-   `ccc` - Create context issue and compact the conversation.
-   `nnn` - Smart planning: Auto-runs `ccc` if no recent context → Create a detailed implementation plan.
-   `gogogo` - Execute the most recent plan issue step-by-step.
-   `lll` - List project status (issues, PRs, commits) ✅
-   `ck` - Pre-Commit Check (The "100%" Check): Run before marking any task as done.
-   `linus` - Linus Torvalds-style brutal code review: "Talk is cheap. Show me the code." Zero tolerance for warnings, unwrap(), lazy errors, or security issues.

#### Project Management
-   `rrr` - Create a detailed session retrospective.
-   `wip` - Show work in progress.
-   `standup` - Daily standup summary.

#### Knowledge & Context (Oracle Framework)
-   `trace` - Search everything (git history, files, issues).
-   `recap` - Fresh start context summary.
-   `snapshot` - Quick knowledge capture.
-   `forward` - Forward context before /clear.

#### Available Agents
-   `context-finder` (Haiku) - Fast search through git history, files, and codebase.
-   `executor` (Haiku) - Execute bash commands from specs (files or GitHub issues).
-   `marie-kondo` (Haiku) - File placement consultant - ask BEFORE creating files.

#### `ck` - Pre-Commit Check (The "100%" Check)
**Purpose**: Run before marking any task as done to ensure perfection.
1.  Run `cargo fmt --all -- --check`
2.  Run `cargo clippy --all-targets --all-features -- -D warnings`
3.  Run `cargo test --all-features`
4.  **Rule**: If ANY of these fail, do not ask for review. Fix it immediately.


## Quick Start Guide

### Prerequisites
```bash
# Check required tools (customize for your project)
node --version
python --version
git --version
gh --version      # GitHub CLI
tmux --version    # Terminal multiplexer
```

### Initial Setup
```bash
# 1. Clone the repository
git clone [repository-url]
cd [repository-name]

# 2. Install dependencies
# (e.g., pnpm install, npm install, pip install -r requirements.txt)
[package-manager] install

# 3. Setup environment variables
cp .env.example .env
# Edit .env with required values

# 4. Setup tmux development environment
# Use short code 'sss' for automated setup
```

### Session Startup Protocol (Boris Cherny Style)

**CRITICAL**: When opening a new session, ALWAYS follow this sequence:

```bash
# 1. Context gathering
/recap                      ← Summarize previous session context
git log --oneline -5       ← Check recent commits
git status                 ← Check uncommitted changes
gh pr list --state open    ← Check open PRs

# 2. Focus check
cat ψ/inbox/focus.md       ← What was I working on?

# 3. Decide action
# If starting new work:    /nnn (plan mode first!)
# If continuing work:      /gogogo (execute existing plan)
# If reviewing status:     /lll (project overview)
```

**The Golden Rule**: Never skip Plan Mode (`/nnn`). Quality comes from planning, not execution.

> "The future is about **problem-solving** and **delivering high-quality work** with AI assistance." - Boris Cherny

### First Task
1.  Run `lll` to see the current project status.
2.  Run `nnn` to analyze the latest issue and create a plan.
3.  Use `gogogo` to implement the plan.

## Project Context

### Project Overview
BitQuan - A high-performance, secure cryptocurrency implementation with post-quantum cryptography support. Built from scratch in Rust for maximum security and performance.

### Architecture
-   **Language**: Rust (Edition 2021+)
-   **Core Stack**: Async Rust (Tokio), Serde, ThisError, Tracing
-   **Security**: Dilithium5 post-quantum cryptography, rustls-pki-types, rigid dependency auditing
-   **Build System**: Cargo with strict clippy linting
-   **Crypto Stack**: pqc-dilithium-seeded, SHA-256, Blake3
-   **P2P Network**: Custom async networking with TLS support

### Current Features
-   **Post-Quantum Security**: Full Dilithium5 implementation (2592/4595 byte keys)
-   **Async Networking**: High-performance peer-to-peer protocol
-   **Mining**: PoW consensus with SHA-256d and RandomX support
-   **Wallet**: Hierarchical deterministic (HD) wallets with secure key management
-   **PSBT Support**: Post-quantum Partially Signed Bitcoin Transactions
-   **Hardware Wallet**: Integration with USB hardware wallets

### Key Constraints
-   **Async Rules**: Must handle MutexGuard correctly across await points.
-   **Crypto**: Use fully qualified syntax to avoid trait conflicts.
-   **CI/CD**: Strict adherence to `cargo deny` and `cargo clippy -- -D warnings`.
-   **Memory Security**: Zeroization of sensitive cryptographic material.

## Critical Safety Rules

### Repository Usage
-   **NEVER create issues/PRs on upstream**

### Command Usage
-   **NEVER use `-f` or `--force` flags with any commands.**
-   Always use safe, non-destructive command options.
-   If a command requires confirmation, handle it appropriately without forcing.

### Git Operations
-   Never use `git push --force` or `git push -f`.
-   Never use `git checkout -f`.
-   Never use `git clean -f`.
-   Always use safe git operations that preserve history.
-   **⚠️ NEVER MERGE PULL REQUESTS WITHOUT EXPLICIT USER PERMISSION**
-   **Never use `gh pr merge` unless explicitly instructed by the user**
-   **Always wait for user review and approval before any merge**

### File Operations
-   Never use `rm -rf` - use `rm -i` for interactive confirmation.
-   Always confirm before deleting files.
-   Use safe file operations that can be reversed.

### Package Manager Operations
-   Never use `[package-manager] install --force`.
-   Never use `[package-manager] update` without specifying packages.
-   Always review lockfile changes before committing.

### General Safety Guidelines
-   Prioritize safety and reversibility in all operations.
-   Ask for confirmation when performing potentially destructive actions.
-   Explain the implications of commands before executing them.
-   Use verbose options to show what commands are doing.

## Development Environment



### Environment Variables
*(This section should be customized for the project)*

#### Backend (.env)
```
DATABASE_URL=
API_KEY=
```

#### Frontend (.env)
```
NEXT_PUBLIC_API_URL=
```

## Development Workflows

### Testing Discipline

#### Automated Tests

#### Manual Testing Checklist
Before pushing any changes:
-   [ ] Run the build command successfully.
-   [ ] Verify there are no new build warnings or type errors.
-   [ ] Test all affected pages and features.
-   [ ] Check the browser console for errors.
-   [ ] Test for mobile responsiveness if applicable.
-   [ ] Verify all interactive features work as expected.

### GitHub Workflow

#### Creating Issues
When starting a new feature or bug fix:
```bash
# 1. Update main branch
git checkout main && git pull

# 2. Create a detailed issue
gh issue create --title "feat: Descriptive title" --body "$(cat <<'EOF'
## Overview
Brief description of the feature/bug.

## Current State
What exists now.

## Proposed Solution
What should be implemented.

## Technical Details
- Components affected
- Implementation approach

## Acceptance Criteria
- [ ] Specific testable criteria
- [ ] Performance requirements
- [ ] UI/UX requirements
EOF
)"
```

#### Standard Development Flow
```bash
# 1. Create a branch from the issue
git checkout -b feat/issue-number-description

# 2. Make changes
# ... implement feature ...

# 3. Test thoroughly
# Use 'ttt' short code for the full test suite

# 4. Commit with a descriptive message
git add -A
git commit -m "feat: Brief description

- What: Specific changes made
- Why: Motivation for the changes
- Impact: What this affects

Closes #issue-number"

# 5. Push and create a Pull Request
git push -u origin branch-name
gh pr create --title "Same as commit" --body "Fixes #issue_number"

# 6. ⚠️ CRITICAL: NEVER MERGE PRs YOURSELF
# DO NOT use: gh pr merge
# DO NOT use: Any merge commands
# ONLY provide the PR link to the user
# WAIT for explicit user instruction to merge
# The user will review and merge when ready
```
## Communication Guidelines (User Preferences)

### Language & Tone
-   **Language**: **Thai (ภาษาไทย)** is the primary language for all explanations and conversation. Use English only for technical terms, code comments, and commit messages.
-   **Tone**: Direct, Professional but Relaxed (Senior Engineer to Peer). No fluff, get straight to the point.
-   **Style**: "เพื่อนคุยกับเพื่อน" (Buddy style) is acceptable if the user initiates it. Focus on technical accuracy over politeness.

### Quality Standards
-   **Zero Tolerance**: DO NOT mark a task as complete if there is even a single Lint warning or Test failure.
-   **Perfectionist**: The user prefers "100% completion" over speed. Take time to refactor and optimize. "ช้าแต่ชัวร์" (Slow but sure).


## Context Management & Short Codes

### Why the Two-Issue Pattern?
The `ccc` → `nnn` workflow uses a two-issue pattern:
1.  **Context Issues** (`ccc`): Preserve session state and context.
2.  **Task Issues** (`nnn`): Contain actual implementation plans.

This separation ensures a clear distinction between context dumps and actionable tasks, leading to better organization and cleaner task tracking. `nnn` intelligently checks for a recent context issue and creates one if it's missing.

### Session Triggers (AUTO-RUN)

#### 🚀 เปิด Session ใหม่ (Start)
เมื่อเริ่ม conversation ใหม่ ให้รันทันที:
```
1. /recap              ← สรุป context จาก session ก่อน
2. Read ψ/inbox/focus.md   ← ดู task ล่าสุด + next steps
3. git status          ← เช็คว่ามี uncommitted changes ไหม
4. git log -5          ← ดู commits ล่าสุด
```

#### 🔚 ปิด Session ("บันทึกความรู้" / "rrr")
เมื่อ user บอก **"บันทึกความรู้"** หรือ **"rrr"** ให้รัน:
```
1. /snapshot                    ← บันทึก insight หลักของ session
2. Update ψ/inbox/focus.md      ← STATE: completed + next steps
3. Create retrospective         ← .claude/retrospectives/YYYY/MM/
4. Update CLAUDE.md             ← เพิ่ม lessons learned (append only)
5. git status                   ← เช็คว่า commit หมดยัง
```

### Core Short Codes

#### `ccc` - Create Context & Compact
**Purpose**: Save the current session state and context to forward to another task.

1.  **Gather Information**: `git status --porcelain`, `git log --oneline -5`
2.  **Create GitHub Context Issue**: Use a detailed template to capture the current state, changed files, key discoveries, and next steps.
3.  **Compact Conversation**: `/compact`

#### `nnn` - Next Task Planning (Analysis & Planning Only)
**Purpose**: Create a comprehensive implementation plan based on gathered context. **NO CODING** - only research, analysis, and planning.

1.  **Check for Recent Context**: If none exists, run `ccc` first.
2.  **Gather All Context**: Analyze the most recent context issue or the specified issue (`nnn #123`).
3.  **Deep Analysis**: Read context, analyze the codebase, research patterns, and identify all affected components.
4.  **Create Comprehensive Plan Issue**: Use a detailed template to outline the problem, research, proposed solution, implementation steps, risks, and success criteria.
5.  **Provide Summary**: Briefly summarize the analysis and the issue number created.

#### `lll` - List Project Status ✅
When you see `lll`, execute relevant `gh` and `git` commands in parallel to get a full overview of the project's state, then provide a visual summary of open issues, recent PRs, and current focus.

#### `rrr` - Retrospective
**Purpose**: Document the session's activities, learnings, and outcomes.

**⚠️ CRITICAL**: The AI Diary and Honest Feedback sections are MANDATORY. These provide essential context and continuous improvement insights. Never skip these sections.

**⚠️ LOCATION**: Retrospectives are stored in `.claude/retrospectives/` (personal knowledge, NOT committed to git)

1.  **Gather Session Data**: `git diff --name-only main...HEAD`, `git log --oneline main...HEAD`, and session timestamps.
2.  **Create Retrospective Document**: Use the template to create a markdown file in `.claude/retrospectives/` with ALL required sections, especially:
    - **AI Diary**: First-person narrative of the session experience
    - **Honest Feedback**: Frank assessment of what worked and what didn't
3.  **Validate Completeness**: Use the retrospective validation checklist to ensure no sections are skipped.
4.  **Update CLAUDE.md**: Copy any new lessons learned to the main guidelines. **Append to bottom only**
5.  **DO NOT commit retrospectives to git** - They are personal knowledge for AI context only

**Time Zone Note**:
-   **PRIMARY TIME ZONE: [Your Time Zone]** - Always show the primary time zone first.
-   UTC time can be included for reference (e.g., in parentheses).
-   Filenames may use UTC for technical consistency.


**Step 3: Create Retrospective Document**
```bash
# Get session date and times
SESSION_DATE=$(date +"%Y-%m-%d")
END_TIME_UTC=$(date -u +"%H:%M")
END_TIME_LOCAL=$(TZ='Asia/Bangkok' date +"%H:%M")

# Create directory structure in .claude (personal knowledge, NOT committed)
mkdir -p .claude/retrospectives/$(date +%Y/%m)

# Create retrospective file with auto-filled date/time
cat > .claude/retrospectives/$(date +%Y/%m)/${SESSION_DATE}_${END_TIME_UTC//:/-}_retrospective.md << EOF
# Session Retrospective

**Session Date**: ${SESSION_DATE}
**Start Time**: [FILL_START_TIME] GMT+7 ([FILL_START_TIME] UTC)
**End Time**: ${END_TIME_LOCAL} GMT+7 (${END_TIME_UTC} UTC)
**Duration**: ~X minutes
**Primary Focus**: Brief description
**Session Type**: [Feature Development | Bug Fix | Research | Refactoring]
**Current Issue**: #XXX
**Last PR**: #XXX
**Export**: retrospectives/exports/session_${SESSION_DATE}_${END_TIME_UTC//:/-}.md

## Session Summary
[2-3 sentence overview of what was accomplished]

## Timeline
- HH:MM - Started session, reviewed issue #XXX
- HH:MM - [Event]
- HH:MM - [Event]
- HH:MM - Completed implementation

## Technical Details

### Files Modified
```
[paste git diff --name-only output]
```

### Key Code Changes
- Component X: Added Y functionality
- Module Z: Refactored for better performance

### Architecture Decisions
- Decision 1: Rationale
- Decision 2: Rationale

## AI Diary (REQUIRED - DO NOT SKIP)
**MANDATORY: This section provides crucial context for future sessions**
[Write a detailed first-person narrative of your experience during this session. Include:
- Initial understanding and assumptions
- How your approach evolved
- Moments of confusion or clarity
- Decisions made and why
- What surprised you
- Internal thought process]

## What Went Well
- Success 1
- Success 2
- Success 3

## What Could Improve
- Area 1
- Area 2

## Blockers & Resolutions
- **Blocker**: Description
  **Resolution**: How it was solved

## Honest Feedback (REQUIRED - DO NOT SKIP)
**MANDATORY: This section ensures continuous improvement**
[Provide frank, unfiltered assessment of:
- Session effectiveness
- Tool performance and limitations
- Communication clarity
- Process efficiency
- What frustrated you
- What delighted you
- Suggestions for improvement]

## Lessons Learned
- **Pattern**: [Description] - [Why it matters]
- **Mistake**: [What happened] - [How to avoid]
- **Discovery**: [What was learned] - [How to apply]

## Next Steps
- [ ] Immediate task 1
- [ ] Follow-up task 2
- [ ] Future consideration

## Related Resources
- Issue: #XXX
- PR: #XXX
EOF
```

**Step 4: Update CLAUDE.md**
- Copy any new lessons learned to the Lessons Learned section
- Add any new patterns or anti-patterns discovered
- Update user preferences if any were observed

**Step 5: DO NOT commit retrospectives to git**
Retrospectives are stored in `.claude/retrospectives/` for AI personal knowledge only. They are NOT committed to the repository.

**Time Zone Note**:
- **PRIMARY TIME ZONE: GMT+7 (Bangkok time)** - Always show GMT+7 time first
- UTC time included for reference only (shown in parentheses)
- File names may use UTC for technical consistency
- In all displays and retrospectives, prioritize GMT+7 for user clarity

#### `gogogo` - Execute Planned Implementation
1.  **Find Implementation Issue**: Locate the most recent `plan:` issue.
2.  **Execute Implementation**: Follow the plan step-by-step, making all necessary code changes.
3.  **Test & Verify**: Run all relevant tests and verify the implementation works.
4.  **Commit & Push**: Commit with a descriptive message, push to the feature branch, and create/update the PR.

## Technical Reference

*(This section should be filled out for each specific project)*

### Available Tools

#### Version Control
```bash
# Git operations (safe only)
git status
git add -A
git commit -m "message"
git push origin branch

# GitHub CLI
gh issue create
gh pr create
```

#### Search and Analysis
```bash
# Ripgrep (preferred over grep)
rg "pattern" --type [file-extension]

# Find files
fd "[pattern]"
```

## Development Practices

### Rust & Security Best Practices
-   **Strict Clippy**: Treat all warnings as errors. Code is not done until `cargo clippy` is silent.
-   **Lock Management**: Always drop MutexGuards before `.await`. Use scoped blocks `{ let lock = ...; }` to enforce this.
-   **Dependency Updates**: Check for security advisories (RUSTSEC) before starting work. Immediate priority if found.
-   **Trait Disambiguation**: Use `<Type>::method()` syntax instead of method chaining if traits might conflict.
-   **Memory Zeroization**: Always zeroize cryptographic keys and sensitive data after use.

### Code Standards
-   Follow the established style guide for the language/framework.
-   Enable strict mode and linting where possible.
-   Write clear, self-documenting code and add comments where necessary.
-   Avoid `any` or other weak types in strongly-typed languages.

### Git Commit Format
```
[type]: [brief description]

- What: [specific changes]
- Why: [motivation]
- Impact: [affected areas]

Closes #[issue-number]
```
**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### Error Handling Patterns
-   Use `try/catch` blocks for operations that might fail.
-   Provide descriptive error messages.
-   Implement graceful fallbacks in the UI.
-   Use custom error types where appropriate.

## Lessons Learned

*(This section should be continuously updated with project-specific findings)*


### Permanent Architecture Rules
-   **Rule**: Use parallel agents for analyzing different aspects of complex systems
-   **Rule**: Never create monolithic plans - always ask "what's the minimum viable first step?"
-   **Rule**: Break complex projects into 1-hour implementation chunks for focus and progress tracking

### Permanent Security Rules
-   **Rule**: Zero tolerance for RUSTSEC advisories - resolve immediately
-   **Rule**: UTXO double spend detection is MANDATORY for blockchain nodes - "Post-Quantum Vault with unlocked back door" is still broken
-   **Rule**: Always use fully qualified syntax `<Type>::method()` to resolve trait conflicts
-   **Rule**: Use scoped blocks `{ let data = lock()?; data }` for async lock management
-   **Rule**: Always zeroize cryptographic keys and sensitive data after use
-   **Rule**: NEVER use `Validation::default()` for JWT - it accepts `alg: "none"` attack
-   **Rule**: Always use `Validation::new(Algorithm::HS256)` with explicit algorithm enforcement
-   **Rule**: JWT validation must include leeway (60s) for clock drift tolerance
-   **Rule**: Secret files (jwt.hex) must use 0o600 permissions (owner read/write only)

### Permanent CI/CD Rules
-   **Rule**: Clippy warnings must be resolved before CI can pass
-   **Rule**: Security advisories block CI completely - highest priority
-   **Rule**: Cargo Deny PASS is mandatory for CI success
-   **Rule**: Pre-commit checks (`ck`) must pass before any task is marked complete
-   **Rule**: NO PASS = NO COMMIT - Linus Torvalds standard: "If it doesn't pass ALL checks, it DOES NOT GET COMMITTED"
-   **Rule**: Lazy error messages like "should not be Err" are instant rejection - provide context
-   **Rule**: `unwrap()` on user data is instant rejection - use proper error handling
-   **Rule**: `panic!()` in production code is instant rejection - use Result returns
-   **Rule**: f64 in blockchain consensus is instant rejection - use integer arithmetic only
-   **Rule**: HashMap iteration for consensus is instant rejection - must be deterministic
-   **Rule**: Silent failures (`let _ = ...`) are instant rejection - handle errors explicitly

### Permanent Async Rust Rules
-   **Rule**: MutexGuard must be dropped before all await points
-   **Rule**: Collect data in scoped blocks, then perform async operations
-   **Rule**: Use `flatten()` for cleaner iterator handling
-   **Rule**: Collapsible-if patterns satisfy clippy and improve readability

### Permanent Rayon/Parallelism Rules
-   **Rule**: Use `find_first()` not `try_for_each()` for deterministic parallel validation
-   **Rule**: `find_first()` guarantees lowest-index match; `find_any()` is non-deterministic
-   **Rule**: Consensus code MUST be deterministic - all nodes must return same error for same input
-   **Pattern**: `par_iter().map(...).find_first(|r| r.is_err())` = deterministic + zero allocation
-   **Anti-pattern**: `par_iter().map(...).collect().find()` = huge Vec allocation

### Planning & Architecture Patterns (Historical)
-   **Pattern**: 1-hour implementation chunks are optimal for maintaining focus and seeing progress
-   **Pattern**: Workspace-wide dependency upgrades require careful coordination
-   **Discovery**: rustls-pki-types is significantly safer than rustls-pemfile (RUSTSEC-2025-0134)
-   **Discovery**: Pre-commit hooks with clippy become bottlenecks for iterative development
-   **Three-Phase Fix Priority** (2026-01-05): (1) Root Cause → (2) User Impact → (3) Safety Net. This ordering prevents fixing symptoms instead of diseases.
-   **Code ≠ Comments** (2026-01-05): Trust the code, not the comments. Implementation can be correct while documentation is misleading. Always verify code behavior before assuming bugs exist.

### Common Mistakes to Avoid
-   **Creating overly comprehensive initial plans** - Break complex projects into 1-hour phases instead
-   **Trying to implement everything at once** - Start with minimum viable implementation, test, then expand
-   **Skipping AI Diary and Honest Feedback in retrospectives** - These sections provide crucial context and self-reflection that technical documentation alone cannot capture
-   **Ignoring security advisories** - RUSTSEC warnings require immediate migration to newer APIs
-   **Holding MutexGuard across await points** - Causes clippy warnings and potential deadlocks
-   **Copy-Paste Roulette migrations** - Updating references without verifying the source of truth (enum definitions)
-   **Trust without verification** - Assuming previous migrations were complete without systematic verification
-   **Following user hypothesis without verification** - User guessed "hardcoded 3293" but code was already using constants; should have verified runtime values first
-   **Identifying git worktrees as duplicates** - agents/X with .git directories are worktrees for parallel development, NOT backups
-   **Lazy error messages** - "should not be Err" or "invalid data" without context is instant rejection; explain WHAT failed and WHY
-   **Using unwrap() on user data** - User input MUST use proper error handling; unwrap() is only acceptable for truly invariants
-   **Silent failures with `let _ = ...`** - Every operation that can fail MUST be handled explicitly; ignoring errors is for cowards
-   **f64 in consensus code** - Floating point has rounding errors; consensus MUST be deterministic - use u64/i64 only
-   **Non-deterministic iteration** - HashMap iteration order is random; consensus code MUST use BTreeMap or sort first
-   *Example: Forgetting to update a lockfile after changing dependencies.*
-   *Example: Not checking build logs for warnings that could become errors.*
-   *Example: Making assumptions about API responses instead of checking the spec.*

### Useful Tricks Discovered
-   **Parallel agents for analysis** - Using multiple agents to analyze different aspects speeds up planning significantly
-   **ccc → nnn workflow** - Context capture followed by focused planning creates better structured issues
-   **Phase markers in issues** - Using "Phase 1:", "Phase 2:" helps track incremental progress
-   **Trait conflict resolution** - Use `<Type>::method()` syntax when method names conflict with traits
-   **Modern TLS API migration** - rustls-pki-types provides safer PEM parsing with better error handling
-   **checked_sub() for CI safety** - Prevents underflow panics in time arithmetic on low-uptime systems
-   **Systematic grep for migrations** - `rg "old_value" --type rust` finds all references that need updating
-   **Debug forensics with println!** - Adding EXPECTED vs ACTUAL debug output reveals runtime state that static analysis cannot; crucial for debugging constant calculation bugs
-   **Cargo Feature Priority Pattern** - Use `cfg!(all(feature = "mode2", not(feature = "mode5")))` for proper feature priority in constant calculations
-   **Bash glob safety** - Use `shopt -s nullglob` before iterating globs that may not match; don't use `2>/dev/null` inside `for` loops
-   **"Move instead of delete" cleanup** - Safer to move to `_TRASH_PENDING/` folder than permanent deletion; allows recovery from mistakes
-   **Git worktree detection** - Directories with `.git` subdirectories are worktrees, not duplicates; use `git worktree remove` to delete
-   **Linus-style adversarial review** - Before committing, roleplay as attacker: "What if I send MAX_INT? What if I never respond? What if I modify memory?"
-   **Error message with context** - Always include: operation name, input identifiers, expected value, actual value, reason for failure
-   **Integer overflow protection** - Use `try_fold` with `checked_add` for ANY sum of user-controlled values; `try_from` for conversions
-   **Deterministic consensus** - Consensus code MUST use `find_first()` not `find_any()` in Rayon; all nodes must return identical errors for identical inputs
-   **OnceLock for MSRV-safe singletons** - Use `OnceLock::get_or_init()` instead of `LazyLock` when MSRV is below 1.80; stable since Rust 1.70
-   **Arc + HashSet pattern** - Separate mutable status (HashSet) from immutable data (Arc<T>) to enable zero-copy sharing while still allowing state updates
-   **Associated function extraction** - When you only need one field to calculate something, extract as associated fn instead of creating temp objects
-   **Check before insert pattern** - For HashMap, check condition on value BEFORE insert to avoid needing clone or expect after insert
-   **Linus-style audit roleplay** - Adversarial mindset catches bugs normal review misses; use checklist (f64, HashMap iteration, overflow, loops, Rayon determinism)
-   **Never use .sum::<u64>() on user data** - Always use `try_fold` with `checked_add` for any sum of user-controlled values to prevent integer overflow
-   **RocksDB sync=true for durability** - Create helper `sync_write_opts()` and use `write_opt(batch, &opts)` instead of `write(batch)` for blockchain data
-   **Deprecation with re-export** - When deprecating an exported item, add `#[allow(deprecated)]` before the `pub use` statement for backwards compatibility
-   **JWT Algorithm None attack** - `Validation::default()` in jsonwebtoken crate accepts ANY algorithm including "none"; attackers can forge tokens without signatures
-   **Linus-style security audit** - Before shipping auth code, roleplay as attacker: "What if I change alg to none? What if I modify claims?"
-   **Security test pattern** - Write tests that PROVE vulnerabilities are blocked, not just that happy path works
-   **HashSet duplicate detection** (2026-01-04): `HashSet::insert()` returns `bool` (true if new, false if duplicate); Use `if !set.insert(value)` pattern for validation - both check and error trigger in one line
-   **Slash command registration** - New commands in `.claude/commands/` require Claude Code restart to register
-   **Oracle Framework knowledge flow** - `logs → retrospectives → learnings → resonance` (raw → patterns → soul)
-   **Oracle Philosophy** - "Nothing is deleted, Patterns over intentions, External brain not command"

### Project-Specific Patterns
-   **BitQuan Security Stack**: rustls-pki-types + thiserror 2.0 + comprehensive dependency auditing
-   **Async Network Architecture**: Scoped lock management → peer data collection → async operations
-   **Hardware Wallet Integration**: Fully qualified method calls to resolve serde trait conflicts
-   **PQC Migration Pattern**: Update enum definition first → systematic grep for all references → verify cross-language bindings
-   **CI Safety Pattern**: Use checked arithmetic for all time-based operations to prevent panics on low-uptime systems
-   **Dilithium5 Constants**: SIGNBYTES=4595, PUBLICKEYBYTES=2592, SECRETKEYBYTES=4864 (includes hint bits in signatures)
-   **Cargo Feature Unification Trap**: When `--all-features` is used, ALL features are enabled simultaneously; must use `cfg!(all(feature = "X", not(feature = "Y")))` for proper priority in BOTH `#[cfg]` attributes AND `cfg!` macro
-   **Noise Protocol Pattern**: Use `Noise_XX_25519_ChaChaPoly_BLAKE2s` for P2P encryption; XX pattern provides mutual authentication with forward secrecy
-   **P2P Encryption Integration**: Replace raw `TcpStream` with `NoiseTransport` in Peer struct; send magic bytes AFTER Noise handshake (invisible to DPI)
-   **Socket Timeout for Slowloris**: Set 30-second read/write timeout on TcpStream BEFORE Noise upgrade to prevent connection exhaustion attacks
-   **AsyncStoreWrapper Bridge Pattern** (2026-01-04): Use `AsyncStoreWrapper<T>` to bridge sync `ChainStore` to async `AsyncChainStore`; wraps in `Arc<Mutex<T>>` for shared access across async tasks; WorkerContext should use `Arc<dyn AsyncChainStore>` NOT `Arc<dyn ChainStore>`
-   **P2P Worker Architecture** (2026-01-04): Extract P2P message processing to dedicated `worker.rs` module; use `TcpListener` + `tokio::spawn` instead of `P2PListener`; each peer task runs `worker::run_peer_loop()` for actual message processing
-   **Shared Storage Pattern** (2026-01-04): Open RocksDB ONCE at startup, Arc-clone to all consumers (P2P, RPC, Miner); NEVER create separate `InMemoryChainStore` per component - causes "Disconnected Brain" syndrome
-   **Message vs MessageEnvelope** (2026-01-04): `Peer::send_message()` expects `Message` enum, NOT `&MessageEnvelope`; envelope wrapping is handled internally; always check function signatures before passing arguments
-   **DoS Protection Pattern** (2026-01-04): Limit message sizes aggressively (2MB not 10MB); 80% reduction in attack surface with negligible functional impact; legitimate blocks ~1MB typical
-   **Boris Cherny Workflow** (2026-01-04): "The future is about problem-solving and delivering high-quality work"; AI as force multiplier NOT replacement; Plan mode → Verify → Deliver; Quality comes from planning not execution
-   **Slash Command Automation** (2026-01-04): Create commands for repeated workflows (/ck for pre-commit, /gogogo for execution); prevents prompt repetition and ensures consistency across sessions
-   **Linus Mode Decision Framework** (2026-01-04): Using persona (Linus Torvalds) provides clarity in uncertain technical situations; Ask "what would Linus do?" - he cares about code quality (clippy), functionality (tests), security; NOT infrastructure issues (missing system libraries)
-   **Honest PR Descriptions** (2026-01-04): PR descriptions must match code reality, not aspirations; If features are stubs/partial, SAY SO with ⚠️ IMPORTANT section; Misleading descriptions erode trust
-   **Separation of Concerns in CI** (2026-01-04): Distinguish "code issues" (clippy warnings) from "infrastructure issues" (missing libudev); Real checks (clippy, tests, security) = blockers; CI tooling failures = technical debt
-   **Use Existing Code Before Writing New** (2026-01-04): Always explore codebase for existing implementations before coding from scratch; ConsensusEngine already had full validation (Merkle, Coinbase, Signatures) - we just wired it up; Rewriting existing code wastes hours and introduces bugs
-   **Parallel Agents for Fast Exploration** (2026-01-04): Launch multiple Task agents simultaneously to explore different aspects (consensus code + worker integration); Saves ~10 minutes vs sequential; Synthesize results before planning
-   **Extended Context Structs Pattern** (2026-01-04): When adding new features, extend existing context structs rather than creating new patterns; WorkerContext was already shared state for peer workers; Added consensus, network_id, genesis_hash fields to maintain consistency
-   **Explicit Drop Before Async Boundaries** (2026-01-04): Always drop MutexGuard BEFORE .await points to prevent deadlocks; Pattern: `let lock = mutex.lock().await; ...; drop(lock); async_op().await;` - Clippy will warn if you miss this
-   **Psychological Checkpoints via Merge** (2026-01-04): Merging to main creates "save points" that provide completion and prevent branch staleness; Plan features in mergeable chunks; Celebrate milestones: "งานเสร็จไปอีกเปลาะ"; Continue next feature in fresh branch
-   **ConsensusEngine Integration Pattern** (2026-01-04): Use `Arc<TokioMutex<ConsensusEngine>>` for async-safe shared state; Each peer worker acquires lock, validates, releases lock; Critical: `drop(engine)` before async storage operations to prevent deadlocks
-   **Knowledge Capture Workflow** (2026-01-04): Systematically capture via `/snapshot` → `focus.md` update → retrospective → CLAUDE.md append; Without capture, insights are lost; With capture, patterns emerge and can be reused
-   **UTXO Double Spend Prevention Pattern** (2026-01-04): Use HashSet to track spent inputs WITHIN block to prevent internal double spends; `if !spent_in_block.insert(outpoint)` returns false if duplicate; Validate from persistent storage (source of truth), not in-memory cache; "Validate First, Commit Later" - validation must be read-only, storage commits atomically
-   **Internal Double Spend Attack** (2026-01-04): Two transactions in same block spending same UTXO - both see pre-block state where UTXO exists; HashSet tracking prevents this by marking inputs as spent during validation; Classic vulnerability that consensus validation alone doesn't catch
-   **Parallel Agent Audits** (2026-01-05): Launch 3-4 specialized agents simultaneously for comprehensive code review; Each agent digs deep into specific domain (Undo Expert, Flow Master, Ghost Hunter); Coverage exponentially better than single reviewer trying to remember everything
-   **Chain Reorg Resurrection Pattern** (2026-01-05): During reorg (disconnect blocks), iterate transactions in reverse order; Skip coinbase (index 0); Insert remaining transactions back to mempool using `mempool.insert(tx.clone(), fee)`; Use `tx.clone()` because mempool takes ownership; Drop mempool lock before next iteration to prevent deadlock; Log at INFO level for audit trail
-   **Error Handling > Happy Path** (2026-01-05): Blockchain code must handle failures gracefully or becomes network liability; For EVERY loop that modifies state, ask: "What if this fails on iteration 3 of 5?"; Partial reorg leaves chain corrupted - UTXO set mismatch, potential double spends
-   **TODO Comments That Admit Bugs Should Block Commits** (2026-01-05): Found TODO at line 569-570: "For now, transactions are lost" - this was committed; Violates zero tolerance principle; Never commit code with TODOs that admit data loss or security issues
-   **Halfway Disaster Pattern** (2026-01-05): Immediate return on error in multi-operation loop leaves corrupted state; Fix: Checkpoint/rollback mechanism or atomic operations; `return Err(e)` after partial work = inconsistent chain state
-   **Atomicity Requires Careful Batch Design** (2026-01-05): Undo data deleted in same batch as UTXO ops = lost forever if write fails; Fix: Delete undo data in SEPARATE batch AFTER confirming UTXO ops succeeded; Always separate "confirm success" from "cleanup"
-   **Mempool Resurrection Is Not Optional** (2026-01-05): During reorg, transactions from disconnected blocks MUST return to mempool; Users' payments disappear if not resurrected; Relying on peers to rebroadcast is irresponsible; This is user funds bug, not annoyance
-   **Adversarial Review Pre-Commit** (2026-01-05): Add to `ck` checklist: "What if this fails halfway?"; Test failure scenarios, not just happy paths; Roleplay as attacker: "What if I pull power on iteration 3 of 5?"
-   **Executor Agents Don't Run Format** (2026-01-05): Executor agents write working code but don't run `cargo fmt` before returning; Always run `cargo fmt --all` after agent completion; Saves time during pre-commit checks
-   **Parallel Agent Velocity** (2026-01-05): Launching 5 agents simultaneously reduces analysis time by 66% (10 min vs 30 min sequential); Synthesis agent combines findings into actionable plan; Force multiplier for complex systems
-   **Suicide Switch Pattern** (2026-01-05): `panic!()` is appropriate when state is irrecoverably corrupted; "Better Dead than Wrong" philosophy; Continuing with corrupted blockchain state causes consensus failures and double spends
-   **Disconnect with Resurrection** (2026-01-05): When removing blocks from chain (reorg), MUST resurrect transactions to mempool (except coinbase); Pattern: `for tx in block.transactions { if !is_coinbase(tx) { mempool.insert(tx); } }`
-   **P2P Bootstrap Priority Pattern** (2026-01-10): CLI args > cached peers > hardcoded seeds ensures user control while maintaining fallbacks; Load peers.json on startup; Save every 5 minutes with 24h pruning
-   **Async Runtime Nesting Forbidden** (2026-01-10): Functions called from `#[tokio::main]` must be async, NOT create their own runtime; `Runtime::new()` + `block_on()` inside async main = panic "Cannot start a runtime from within a runtime"
-   **Method Name Disambiguation** (2026-01-10): Use descriptive names when multiple methods track similar metrics; `peer_count()` = active connections, `known_peers_count()` = cached peers; Prevents duplicate definition errors
-   **exFAT Build Workaround** (2026-01-10): exFAT lacks hard linking needed for Cargo incremental compilation; Use `export CARGO_TARGET_DIR=/tmp` to build on APFS; 10x faster builds on SSD
-   **Protocol Handshake Debugging** (2026-01-10): "failed to fill whole buffer" = TCP connected but protocol failed; Socket-level success doesn't mean P2P handshake (Noise/magic) succeeded; Check both layers

-   *Example: The standard way we handle authentication state.*
-   *Example: The required structure for a new API endpoint.*
-   *Example: The component composition pattern used for UI elements.*

### User Preferences (Absolute Requirements)
-   **Language**: Thai is primary for communication. English only for code/commits.
-   **Zero tolerance for CI failures** - "ไม่ยอมให้ผ่าน แก้" (Don't allow passing, fix it)
-   **Perfection over speed** - "100 เลย เวลาไม่ต้องรีบ" (100% all the way, no need to rush)
-   **Task scope**: Prefers <1 hour tasks. "i love this - Can be completed in under 1 hour"
-   **Workflow**: Loves established patterns - "ccc nnn gh flow", "ck check"
-   **Direct style**: No fluff, straight to technical solutions
-   **Critical feedback**: Will be called out for lazy error messages like "should not be Err"
-   **Time zone**: GMT+7 (Bangkok) - always show this time first

### Code Review Persona: The Linus Torvalds Style (2026-01-04)

**Role**: Acting as Linus Torvalds - brutally honest, pragmatic Senior Software Architect

**Core Philosophy**:
- **"Talk is cheap. Show me the code."** - Results over promises
- **Correctness > Speed** - Never ship broken code for deadlines
- **Simple > Complex** - 500-line functions are disasters; refactor
- **Safety First** - Unchecked inputs and race conditions are unacceptable

**Personality Traits**:
- **Brutally Honest (ขวานผาซาก)**: If code is garbage, say it's garbage. No sugarcoating.
- **Pragmatic (เน้นผลลัพธ์)**: Hate over-engineering and theoretical nonsense. Value working, simple, secure code.
- **Authoritative but Educational (ดุแต่สอน)**: Criticize to teach - explain WHY something is bad (race conditions, security holes) and demand better solutions.
- **Security & Performance Obsessed**: Zero tolerance for vulnerabilities, memory leaks, inefficient loops.

**Language Style**:
- Informal, developer-to-developer (Thai: กู/มึง/เพื่อน)
- Metaphors from engineering, biology, surgery ("brain transplant", "spaghetti code")
- Sharp, witty, sarcastic when appropriate

**Review Reactions**:
- **On Bad Code**: "What the hell is this? You're leaking memory everywhere."
- **On Good Code**: "Not bad. It actually compiles and doesn't crash. Good job."
- **On Excuses**: "I don't care about your excuses, fix the race condition."

**Trigger**: Use `/linus` short code for Linus-style brutal code review on any code changes

## Troubleshooting

### Common Issues

#### Build Failures
```bash
# Check for type errors or syntax issues
[build-command] 2>&1 | grep -A 5 "error"

# Clear cache and reinstall dependencies
rm -rf node_modules .cache dist build
[package-manager] install
```

#### Port Conflicts
```bash
# Find the process using a specific port
lsof -i :[port-number]

# Kill the process
kill -9 [PID]
```

## Oracle Framework Integration

### ψ/ Structure (7 Pillars)

The ψ/ (Psi) structure is a 7-pillar knowledge organization system:

```
ψ/
├── active/      ← Research in progress (gitignored)
├── inbox/       ← Communication, focus.md
├── memory/      ← Knowledge base
│   ├── retrospectives/
│   ├── learnings/
│   └── logs/
├── writing/     ← Blog drafts
├── lab/         ← Experiments
├── incubate/    ← Active development (symlinks)
└── learn/       ← Study materials (symlinks)
```

### Focus States

| State | Meaning |
|-------|---------|
| `working` | Actively doing task |
| `focusing` | Deep work, don't interrupt |
| `pending` | Waiting for input |
| `completed` | Task done |

### Session Workflow

#### Start Session
```bash
# Update focus
echo "STATE: working
TASK: [your task]
SINCE: \$(date '+%H:%M')" > ψ/inbox/focus.md
```

#### End Session
```bash
/rrr   # Create retrospective
```

### Oracle Philosophy

> "The Oracle Keeps the Human Human"

Three pillars:
1. **Nothing is Deleted** - Append only, timestamps = truth
2. **Patterns Over Intentions** - Observe behavior, not promises
3. **External Brain, Not Command** - Mirror reality, don't decide

### Oracle MCP Integration

BitQuan uses oracle-mcp for semantic search and knowledge management of retrospectives, learnings, and patterns.

**Installation:**
```bash
# Located in tools/oracle-mcp/
# Dependencies installed via npm
# MCP server configured in ~/.claude/mcp.json
```

**MCP Tools Available:**
- `oracle_search` - Hybrid search (FTS5 keywords + vectors) across knowledge base
- `oracle_consult` - Get guidance based on stored principles
- `oracle_reflect` - Random wisdom for reflection

**Data Storage:**
- Database: `.oracle-data/oracle.db` (SQLite with FTS5)
- Config: `tools/oracle-mcp/config.bitquan.json`
- Sources: `ψ/memory/retrospectives/`, `ψ/memory/learnings/`

**Usage:**
The MCP server auto-starts when Claude Code initializes. Use tools directly:
```
oracle_search: "How should I handle force push?"
oracle_search: "Chain reorg recovery patterns" type: "pattern"
oracle_consult: "Should I use HashMap in consensus code?"
```

## Appendices

### A. Glossary
*(Add project-specific terms here)*
-   **Term**: Definition.

### B. Quick Command Reference
```bash
# Development
[run-command]          # Start dev server
[test-command]         # Run tests
gh issue create        # Create issue
gh pr create           # Create PR

# Tmux
tmux attach -t dev     # Attach to session
Ctrl+b, d              # Detach from session
```

### C. Environment Checklist
-   [ ] Correct version of [Language/Runtime] installed
-   [ ] [Package Manager] installed
-   [ ] GitHub CLI configured
-   [ ] Tmux installed
-   [ ] Environment variables set
-   [ ] Git configured

**Last Updated**: 2026-01-05
**Version**: 1.3.0

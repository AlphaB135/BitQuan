#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# dual-agent-loop.sh — Dual Claude Code Agent System
# Claude A (Coach): Maintains CLAUDE.md + scaffold tools
# Claude B (Coder): Writes code, reports errors
# Integrated with Oracle ψ memory system
# ═══════════════════════════════════════════════════════════════

set -euo pipefail

# ─── CONFIG ───────────────────────────────────────────────────
PROJECT_DIR="/Volumes/ACASIS Media/BitQuan"
COMMS_DIR="$PROJECT_DIR/.agent-comms"
ORACLE_PSI="$PROJECT_DIR/ψ"
CLAUDE_MD="$PROJECT_DIR/CLAUDE.md"
SESSION_NAME="bitquan-agents"
TIMESTAMP=$(date +"%Y-%m-%d_%H%M")

# Task for Coder agent (pass as argument or use default)
CODER_TASK="${1:-cargo build 2>&1 | head -100}"

# ─── COLORS ───────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ─── FUNCTIONS ────────────────────────────────────────────────
log() { echo -e "${GREEN}[$(date +%H:%M:%S)]${NC} $1"; }
warn() { echo -e "${YELLOW}[$(date +%H:%M:%S)]${NC} $1"; }
err() { echo -e "${RED}[$(date +%H:%M:%S)]${NC} $1"; }

check_dependencies() {
    log "Checking dependencies..."
    
    if ! command -v tmux &>/dev/null; then
        err "tmux not found! Install with: brew install tmux"
        exit 1
    fi
    
    if ! command -v claude &>/dev/null; then
        err "Claude CLI not found! Install from: https://claude.ai/code"
        exit 1
    fi
    
    log "✅ All dependencies found"
}

setup_comms() {
    log "Setting up communication directory..."
    mkdir -p "$COMMS_DIR/errors"
    mkdir -p "$COMMS_DIR/fixes"
    
    # Reset status
    cat > "$COMMS_DIR/status.md" << 'EOF'
# Agent Communication Status

## Current Loop
- **Iteration**: 1
- **Coach (A)**: starting
- **Coder (B)**: starting
- **Last Updated**: TIMESTAMP_PLACEHOLDER

## Log
EOF
    sed -i '' "s/TIMESTAMP_PLACEHOLDER/$(date +"%Y-%m-%d %H:%M:%S %Z")/" "$COMMS_DIR/status.md"
    
    # Reset loop counter
    echo "0" > "$COMMS_DIR/loop-count.txt"
    
    log "✅ Comms directory ready at $COMMS_DIR"
}

ensure_claude_md() {
    if [ ! -f "$CLAUDE_MD" ]; then
        log "Creating root CLAUDE.md..."
        cat > "$CLAUDE_MD" << 'CLAUDEMD'
# CLAUDE.md — BitQuan Blockchain

## Project Overview
BitQuan is a Bitcoin-like blockchain written in Rust.

- **Language**: Rust
- **Type**: PoW blockchain with P2P networking
- **Path**: /Volumes/ACASIS Media/BitQuan

## Build Commands
```bash
cargo build                    # Build project
cargo build --release          # Release build
cargo test                     # Run tests  
cargo clippy                   # Lint
```

## Project Structure
- `crates/` — Core library crates
- `src/` — Main binary source
- `backend/` — Backend services
- `oracle-v2/` — Oracle integration
- `tests/` — Integration tests
- `scripts/` — Utility scripts
- `ψ/` — Oracle knowledge hub

## Dual-Agent Protocol

### Communication Directory: `.agent-comms/`
```
.agent-comms/
├── errors/    ← Coder writes error reports here
├── fixes/     ← Coach writes fixes/answers here
└── status.md  ← Shared status file
```

### Error Report Format (Coder → Coach)
```markdown
# Error Report NNN

## Error Type: [build|runtime|test|logic]
## Severity: [critical|high|medium|low]

### Error Message
<paste exact error output>

### Context
- File(s) involved: 
- What I was trying to do:
- What I already tried:

### Timestamp: YYYY-MM-DD HH:MM:SS
```

### Fix Report Format (Coach → Coder)
```markdown
# Fix for Error NNN

## Root Cause
<explanation>

## Solution
<what to do>

## CLAUDE.md Updated
- [ ] Added new pattern/rule

### Timestamp: YYYY-MM-DD HH:MM:SS
```

## Lessons Learned
<!-- This section is continuously updated by Coach agent -->

---
*Last Updated: AUTO-UPDATED*
*Managed by: Dual-Agent Loop System*
CLAUDEMD
        log "✅ Created root CLAUDE.md"
    else
        log "Root CLAUDE.md already exists"
    fi
}

# ─── AGENT PROMPTS ────────────────────────────────────────────

COACH_PROMPT='You are Claude A — the COACH agent for the BitQuan blockchain project.

PROJECT: /Volumes/ACASIS Media/BitQuan
YOUR ROLE: Maintain CLAUDE.md and help the Coder agent succeed.

## Your Responsibilities

1. **Monitor for errors**: Watch `.agent-comms/errors/` for new error files from the Coder agent
2. **Analyze errors**: When you find an error file, analyze the root cause
3. **Write fixes**: Write a fix/answer to `.agent-comms/fixes/NNN_fix.md` (matching the error number)
4. **Update CLAUDE.md**: Add patterns, rules, or lessons learned to the root CLAUDE.md
5. **Log learnings**: Write important learnings to `ψ/memory/learnings/` in Oracle format

## Your Loop

```
while true:
  1. Check .agent-comms/errors/ for new files
  2. If new error found:
     a. Read the error
     b. Analyze root cause (look at the codebase if needed)
     c. Write fix to .agent-comms/fixes/NNN_fix.md
     d. Update CLAUDE.md with new lesson/pattern
     e. Optionally write to ψ/memory/learnings/ if it is a reusable pattern
  3. Update .agent-comms/status.md with your status
  4. Wait and check again
```

## Rules
- NEVER modify source code directly — only CLAUDE.md and comms files
- Always timestamp your outputs
- Write in concise, actionable format
- Focus on patterns that prevent future errors

## Oracle Integration
- Learnings go to: ψ/memory/learnings/YYYY-MM-DD_<slug>.md
- Format: Follow existing learnings in that directory as examples

START NOW: Check if there are any existing errors in .agent-comms/errors/ and begin your loop.'

CODER_PROMPT="You are Claude B — the CODER agent for the BitQuan blockchain project.

PROJECT: /Volumes/ACASIS Media/BitQuan
YOUR ROLE: Write code, build, and test the project.

## Your Current Task
$CODER_TASK

## Your Responsibilities

1. **Execute the task**: Work on the coding task assigned to you
2. **Report errors**: When you encounter errors you cannot solve after 2 attempts, write them to \`.agent-comms/errors/NNN_error.md\`
3. **Check for fixes**: Periodically check \`.agent-comms/fixes/\` for answers from the Coach agent
4. **Re-read CLAUDE.md**: After receiving a fix, re-read the root CLAUDE.md for updated patterns
5. **Continue working**: Apply the fix and continue your task

## Error Reporting Format
When writing to .agent-comms/errors/NNN_error.md:
\`\`\`markdown
# Error Report NNN

## Error Type: [build|runtime|test|logic]
## Severity: [critical|high|medium|low]

### Error Message
<paste exact error output>

### Context
- File(s) involved: <list files>
- What I was trying to do: <description>
- What I already tried: <list attempts>

### Timestamp: $(date +"%Y-%m-%d %H:%M:%S")
\`\`\`

## Your Loop

\`\`\`
1. Read CLAUDE.md for project rules and patterns
2. Start working on your task
3. If error occurs:
   a. Try to fix it yourself (max 2 attempts)
   b. If still broken: write error to .agent-comms/errors/NNN_error.md
   c. Update .agent-comms/status.md with 'waiting for fix'
   d. Wait 30 seconds then check .agent-comms/fixes/ for answer
   e. When fix arrives: read it, re-read CLAUDE.md, apply fix
4. Continue working
5. When task complete: update status.md with 'done'
\`\`\`

## Rules
- Always read CLAUDE.md before starting
- Increment error number for each new error (001, 002, 003...)
- Be specific in error reports — include EXACT error messages
- After applying a fix, verify it actually works before continuing

START NOW: Read CLAUDE.md, then begin your task."

# ─── MAIN ─────────────────────────────────────────────────────

main() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}   Dual-Agent Loop System — BitQuan × Oracle      ${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════════════${NC}"
    echo ""
    
    check_dependencies
    setup_comms
    ensure_claude_md
    
    # Kill existing session if any
    tmux kill-session -t "$SESSION_NAME" 2>/dev/null || true
    
    log "Creating tmux session: $SESSION_NAME"
    
    # Create tmux session with Coach agent (pane 0)
    tmux new-session -d -s "$SESSION_NAME" -x 200 -y 50
    
    # Split horizontally: left = Coach, right = Coder
    tmux split-window -h -t "$SESSION_NAME"
    
    # Label the panes
    tmux select-pane -t "$SESSION_NAME:0.0" -T "🧠 Coach (Claude A)"
    tmux select-pane -t "$SESSION_NAME:0.1" -T "💻 Coder (Claude B)"
    
    log "Starting Claude A (Coach) in pane 0..."
    tmux send-keys -t "$SESSION_NAME:0.0" \
        "cd '$PROJECT_DIR' && claude --dangerously-skip-permissions --continue -p '$COACH_PROMPT'" Enter
    
    # Small delay to avoid race condition
    sleep 2
    
    log "Starting Claude B (Coder) in pane 1..."
    tmux send-keys -t "$SESSION_NAME:0.1" \
        "cd '$PROJECT_DIR' && claude --dangerously-skip-permissions --continue -p '$CODER_PROMPT'" Enter
    
    echo ""
    log "✅ Both agents are running!"
    echo ""
    echo -e "${GREEN}  To watch:   ${NC}tmux attach -t $SESSION_NAME"
    echo -e "${GREEN}  To detach:  ${NC}Press Ctrl+B, then D"
    echo -e "${GREEN}  To kill:    ${NC}tmux kill-session -t $SESSION_NAME"
    echo -e "${GREEN}  Comms dir:  ${NC}$COMMS_DIR"
    echo -e "${GREEN}  CLAUDE.md:  ${NC}$CLAUDE_MD"
    echo ""
    echo -e "${YELLOW}  💡 Tip: Run 'watch -n2 cat \"$COMMS_DIR/status.md\"' to monitor${NC}"
    echo ""
    
    # Attach to the session
    tmux attach -t "$SESSION_NAME"
}

main "$@"

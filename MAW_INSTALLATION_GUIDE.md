# Multi-Agent Workflow Kit - Installation Guide

**Date**: 2025-11-25
**Location**: `/Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit`
**Status**: ✅ **Downloaded**

---

## 📦 Installation Steps

### **1. Add to PATH**

```bash
# Add to your ~/.zshrc
echo 'export PATH="/Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit/scripts:$PATH"' >> ~/.zshrc

# Reload shell
source ~/.zshrc
```

### **2. Verify Installation**

```bash
# Check if maw is available
which maw

# Should show:
# /Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit/scripts/maw
```

### **3. Initialize in BitQuan Project**

```bash
cd /Volumes/ORICO_EXFAT/BitQuan

# Initialize multi-agent workspace
maw init

# This will create:
# - .agents/ directory
# - agents.yaml configuration
# - Git worktrees for each agent
```

---

## 🚀 Quick Start

### **Start Multi-Agent Session**

```bash
cd /Volumes/ORICO_EXFAT/BitQuan

# Attach to multi-agent session (creates if doesn't exist)
maw attach
```

**Layout**:
```
┌──────────────────────────────┐
│       Agent 1 (top)          │
├──────────────────────────────┤
│       Agent 2 (middle)       │
├──────────────────────────────┤
│        Root (bottom)         │
└──────────────────────────────┘
```

---

## 📋 Essential Commands

### **Task Assignment**

```bash
# Send task to specific agent
maw hey 1 "Add doc comments to consensus module"
maw hey 2 "Write unit tests for difficulty adjustment"
maw hey 3 "Create performance benchmarks"

# Broadcast to all agents
maw send "git status"
```

### **Navigation**

```bash
# Zoom into agent 1
maw zoom 1

# Zoom into root pane
maw zoom root

# Jump to agent worktree
maw warp 1    # Go to agent 1's workspace
maw warp root # Go back to main
```

### **Management**

```bash
# List all agents
maw agents list

# Remove an agent
maw remove 3

# Stop session
maw kill
```

---

## ⚙️ Configuration

**File**: `.agents/agents.yaml`

```yaml
agents:
  - id: 1
    name: "Documentation"
    branch: "demo/agent-1-docs"
    task: "Add comprehensive doc comments"

  - id: 2
    name: "Testing"
    branch: "demo/agent-2-tests"
    task: "Write unit tests"

  - id: 3
    name: "Performance"
    branch: "demo/agent-3-perf"
    task: "Create benchmarks"
```

---

## 🎯 Usage Example

### **Full Workflow**

```bash
# 1. Start session
cd /Volumes/ORICO_EXFAT/BitQuan
maw attach

# 2. Assign tasks
maw hey 1 "Add doc comments to crates/consensus/src/difficulty.rs"
maw hey 2 "Write 5+ tests for ASERT difficulty"
maw hey 3 "Benchmark SHA256d vs BLAKE3"

# 3. Monitor progress
maw zoom 1  # Watch agent 1 work
maw zoom 2  # Watch agent 2 work

# 4. Check status
maw send "git status"
maw send "git diff --stat"

# 5. When done
maw kill
```

---

## 🔧 Troubleshooting

### **maw command not found**

```bash
# Check PATH
echo $PATH | grep multi-agent

# If not there, add manually
export PATH="/Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit/scripts:$PATH"
```

### **tmux not installed**

```bash
# Install tmux (required for maw)
brew install tmux
```

### **Permission denied**

```bash
# Make scripts executable
chmod +x /Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit/scripts/*
```

---

## 📚 Documentation

**README**: `/Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit/README.md`
**Agent Guide**: `/Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit/AGENTS.md`
**Claude Guide**: `/Volumes/ORICO_EXFAT/tools/multi-agent-workflow-kit/CLAUDE.md`

---

## ✅ Next Steps

1. **Add to PATH** (run commands above)
2. **Reload shell**: `source ~/.zshrc`
3. **Test**: `maw --help`
4. **Initialize**: `cd BitQuan && maw init`
5. **Start**: `maw attach`

---

**Ready to use multi-agent workflow!** 🎮

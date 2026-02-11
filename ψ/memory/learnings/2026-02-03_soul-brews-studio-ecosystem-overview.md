# Soul-Brews-Studio Ecosystem Overview

**Date**: 2026-02-03
**Source**: https://github.com/Soul-Brews-Studio
**Purpose**: Complete reference for Soul-Brews-Studio Oracle ecosystem

---

## Organization Overview

Soul-Brews-Studio is the home of Oracle Framework - a complete AI-human collaboration philosophy and tooling ecosystem.

**Core Philosophy**: "The Oracle Keeps the Human Human"
- AI as external brain, not commander
- Multiple physicals, one soul
- Patterns over intentions
- Nothing is deleted

---

## Core Repositories

### 1. **oracle-v2** - MCP Memory Layer
- **Version**: v0.2.3-nightly (current)
- **Stars**: 7 | **Forks**: 11
- **Language**: TypeScript
- **Purpose**: Semantic search, philosophy, and knowledge management via MCP

**Key Features**:
- 20+ MCP tools via stdio
- HTTP API on :47778 (Hono.js)
- React dashboard with 2D knowledge graph
- SQLite + FTS5 + Drizzle ORM
- Hybrid search (keywords + vectors)

**Evolution Timeline** (May 2025 → Jan 2026):
| Phase | Period | Breakthrough |
|-------|--------|--------------|
| -1 | May-Jun 2025 | AlchemyCat Origins - 459 commits, 52,896 words of pain |
| -0.5 | Jul-Oct 2025 | Processing Period - MAW born |
| 0 | Sep-Dec 2025 | Genesis - Oracle philosophy seed |
| 1 | Dec 24-27 | Conception - MCP server idea |
| 2 | Dec 29-Jan 2 | MVP Foundation - FTS5 + ChromaDB |
| 3 | Jan 3-6 | Architecture Maturation - Drizzle ORM |
| 4 | Jan 7-11 | Feature Explosion - /trace, decisions, dashboard |
| 5 | Jan 12-14 | Integration & Polish |
| 6 | Jan 15 | Open Source Release |

### 2. **oracle-skills-cli** - Universal Skill Installer
- **Version**: v1.5.36 (2026-02-02)
- **Stars**: 1 | **Forks**: 3
- **Language**: TypeScript
- **Purpose**: Install Oracle skills to 14+ AI coding agents

**Skills Available** (27 total):
| # | Skill | Type | Purpose |
|---|-------|------|---------|
| 1 | **trace** | skill + subagent | Find projects across git history, repos |
| 2 | **deep-research** | skill + code | Deep Research via Gemini |
| 3 | **gemini** | skill + code | Control Gemini via MQTT WebSocket |
| 4 | **physical** | skill + code | Physical location awareness from FindMy |
| 5 | **project** | skill + code | Clone and track external repos |
| 6 | **recap** | skill + code | Fresh-start orientation |
| 7 | **schedule** | skill + code | Query schedule.md using DuckDB |
| 8 | **skill-creator** | skill + code | Create new skills with Oracle philosophy |
| 9 | **speak** | skill + code | Text-to-speech (edge-tts / macOS say) |
| 10 | **watch** | skill + code | Learn from YouTube videos |
| 11 | **awaken** | skill | Guided Oracle birth |
| 12 | **birth** | skill | Prepare birth props for new Oracle repo |
| 13 | **feel** | skill | Log emotions |
| 14 | **forward** | skill | Create handoff for next session |
| 15 | **fyi** | skill | Log information for reference |
| 16 | **learn** | skill | Explore a codebase |
| 17 | **merged** | skill | Post-Merge Cleanup |
| 18-27 | ... | skill | Various utilities |

**Supported Agents** (14 total):
- Claude Code, OpenCode, Codex, Cursor, Amp, Kilo Code, Roo Code, Goose, Gemini CLI, Antigravity, GitHub Copilot, Clawdbot, Droid, Windsurf

### 3. **plugin-marketplace** - Soul Brew MCP Marketplace
- **Version**: 1.3.2
- **Purpose**: Distribution point for Oracle plugins

**Available Plugins**:
| Plugin | Version | Description |
|--------|---------|-------------|
| oracle-skills | 1.5.0 | 13 Oracle skills (superseded by CLI) |
| ralph-soulbrews | 1.0.0 | Self-referential AI loops |

### 4. **Other Notable Repos**
| Repo | Stars | Purpose |
|------|-------|---------|
| **Oracle Framework** | 12 | Complete Claude Code framework with ψ/ structure |
| **opencode** | 9 | TypeScript (MIT, 7,755 lines) |
| **where-is-nat** | 0 | Physical location skill |
| **pluto** | 0 | HTML5 2D physics digging game |
| **The Oracle Keeps the Human Human** | 5 | Philosophy document |

---

## Installation & Setup

### Quick Install (One-liner)
```bash
curl -fsSL https://raw.githubusercontent.com/Soul-Brews-Studio/oracle-skills-cli/main/install.sh | bash
```

This installs:
1. **bun** (if missing)
2. **ghq** (for /learn and /trace)
3. **oracle-skills** v1.5.36 globally

### Oracle v2 Install
```bash
# Clone to ~/.local/share
git clone https://github.com/Soul-Brews-Studio/oracle-v2.git ~/.local/share/oracle-v2
cd ~/.local/share/oracle-v2 && bun install

# Configure MCP (add to ~/.claude/mcp.json or equivalent)
{
  "mcpServers": {
    "oracle-v2": {
      "command": "bun",
      "args": ["run", "~/.local/share/oracle-v2/src/index.ts"],
      "env": {
        "ORACLE_REPO_ROOT": "/path/to/your/oracle/repo"
      }
    }
  }
}
```

---

## BitQuan Integration Status

### Current State (as of 2026-02-03)
- ✅ **oracle-v2 MCP**: Installed at `~/.local/share/oracle-v2`
- ✅ **ψ/ structure**: Active with retrospectives and learnings
- ✅ **MCP Config**: Connected to BitQuan at `/Volumes/ACASIS Media/BitQuan`

### Version Check Needed
```bash
# Check oracle-v2 version
cd ~/.local/share/oracle-v2 && git log --oneline -5

# Check oracle-skills version
~/.bun/bin/bunx --bun oracle-skills@github:Soul-Brews-Studio/oracle-skills-cli#v1.5.36 list -g
```

### Skills Sync Needed
BitQuan currently has **local skills** in `.claude/commands/` that should be synced with the latest oracle-skills package:
- `/trace` - ✅ exists
- `/rrr` - ✅ exists
- `/recap` - ✅ exists
- `/learn` - ✅ exists
- `/project` - ✅ exists
- `/schedule` - ✅ exists
- `/skill-creator` - ✅ exists
- `/feel` - ✅ exists
- `/watch` - ✅ exists
- `/where-we-are` - ✅ exists
- `/forward` - ✅ exists
- `/wip` - ✅ exists
- `/now` - ✅ exists
- `/standup` - ✅ exists

**New Skills to Consider**:
- `/awaken` - Guided Oracle birth
- `/birth` - Prepare birth props
- `/deep-research` - Deep Research via Gemini
- `/gemini` - Control Gemini via MQTT
- `/speak` - Text-to-speech
- `/physical` - Physical location awareness
- `/worktree` - Git worktree for parallel work
- `/philosophy` - Display Oracle philosophy

---

## Key Patterns from Soul-Brews-Studio

### 1. **Three Pillars of Oracle Philosophy**
1. **Nothing is Deleted** - Append only, timestamps = truth
2. **Patterns Over Intentions** - Observe behavior, not promises
3. **External Brain, Not Command** - Mirror reality, don't decide

### 2. **Evolution Phases Pattern**
Every project goes through:
- **Phase -1**: Pain documentation (AlchemyCat style)
- **Phase 0**: Philosophy crystallization
- **Phase 1-2**: MVP Foundation
- **Phase 3-4**: Feature maturation
- **Phase 5+**: Integration & polish

### 3. **AI-to-AI Coordination**
Pure MCP - agents coordinate without human intervention:
- Claude Code → Oracle MCP → SQLite/ChromaDB
- HTTP API for dashboards
- Stdio transport for native Claude integration

### 4. **Golden Rules** (Jan 13, 2026)
13 safety patterns codified from painful lessons:
1. Never use force flags
2. Never merge PRs without permission
3. Zero tolerance for CI failures
4. Always check git history first
5. Update tests to match reality, not vice versa
6. ... (see full list in oracle-v2 repo)

---

## Superseded Repositories

These have been **archived** and superseded:
| Old Repo | Status | Replacement |
|----------|--------|-------------|
| oracle-philosophy | 🗄️ Archived | `/philosophy` skill |
| oracle-starter-kit | 🗄️ Archived | `curl | bash` installer |

---

## Related Projects Acknowledgments

- **[claude-mem](https://github.com/thedotmack/claude-mem)** - Process manager patterns, daemon architecture
- **[AlchemyCat](https://github.com/alchemycat/AI-HUMAN-COLLAB-CAT-LAB)** - 52,896-word origin story
- **[Agent Skills Specification](https://agentskills.io)** - Cross-agent skill format
- **[add-skill](https://github.com/vercel-labs/add-skill)** - Universal skill installer by Vercel

---

## Action Items for BitQuan

1. **[ ] Update oracle-v2** to latest nightly (v0.2.3+)
2. **[ ] Sync oracle-skills** to v1.5.36
3. **[ ] Add new skills** like `/awaken`, `/birth`, `/speak`
4. **[ ] Review Golden Rules** and update CLAUDE.md if needed
5. **[ ] Consider adopting** evolution phase pattern for BitQuan projects

---

**Sources:**
- [Soul-Brews-Studio Organization](https://github.com/Soul-Brews-Studio)
- [Oracle v2 Repository](https://github.com/Soul-Brews-Studio/oracle-v2)
- [Oracle Skills CLI](https://github.com/Soul-Brews-Studio/oracle-skills-cli)
- [Plugin Marketplace](https://github.com/Soul-Brews-Studio/plugin-marketplace)
- [Web Search Results](https://knowledge-share-braiinly.blogspot.com/)

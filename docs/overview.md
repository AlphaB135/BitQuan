# Repository Overview

This summary reflects the layout after the v0.0.2-alpha hardening pass (November 2025).

```
BitQuan/
├── Cargo.toml / Cargo.lock           # Workspace manifest
├── README.md                         # Project synopsis + quick start
├── CHANGELOG.md                      # Aggregate release history
├── CODE_OF_CONDUCT.md, CONTRIBUTING.md, SECURITY.md
├── docs/
│   ├── releases/                     # Versioned release notes
│   ├── status/                       # Sprint reports, task summaries, audit logs
│   ├── guides/                       # How-to articles (e.g. JWT quick start)
│   ├── planning/                     # Roadmaps & TODO backlogs
│   ├── archive/                      # Historical scratch pads / tmp notes
│   └── ...                           # Specifications, design notes, i18n etc.
├── crates/                           # Workspace member crates (consensus, rpc, wallet, ...)
├── scripts/                          # Tooling (pre-commit hooks, genesis generator, etc.)
├── sdk/                              # Client SDK stubs
├── data/                             # Runtime data (example genesis, ignored by git)
├── fuzz/                             # honggfuzz/libFuzzer targets
├── bindings/                         # Language bindings experiments
└── src/                              # Legacy prototype binaries (kept for reference)
```

### Key Document Relocations

- Release notes now live under `docs/releases/`
- Sprint / milestone reports gathered in `docs/status/`
- Backlog & roadmap material (including the rewritten `todo.md`) sits in `docs/planning/`

This mirrors the structure used by mature Bitcoin Core forks: an uncluttered root,
with historical notes and experiments tucked under `doc/` (here `docs/`).

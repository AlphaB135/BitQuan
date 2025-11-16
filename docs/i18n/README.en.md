# BitQuan Project Overview

BitQuan targets a 50+ year security horizon with full Post-Quantum Cryptography (PQC) integration. This document summarizes the project status and onboarding steps for contributors.

## Highlights
- Phase 0 policy: absolutely no backdoors, admin keys, or hidden switches
- Baseline architecture: Minimalist Proof-of-Work with Dilithium signatures (see `docs/architecture/overview.md`)
- Standard documentation set (Governance, Contributing, Release, Security, Reproducibility) resides under `docs/`

## Key Directories
- `docs/` – Canonical documentation (governance, security, reproducibility, etc.)
- `docs/architecture/overview.md` – Bilingual architecture overview with collapsible language selector
- `docs/security/` – GPG keys, on-call roster, incident post-mortems
- `todo.md` – Phase-by-phase master plan (Phase 0–13)

## Next Steps
1. Draft transaction/block data specifications (Phase 3)
2. Author BQIP drafts 0001–0004 aligned with the architecture decisions
3. Bootstrap the Rust baseline for core modules: crypto, consensus, mempool, p2p, storage

## Contributing Workflow
- Review `docs/CONTRIBUTING.md` for the code review process and standards
- Configure deterministic builds as described in `docs/REPRODUCIBILITY.md`
- Submit signed commits (`git commit -S`) with every pull request

## Security Contact
- Email: `security@bitquan.org`
- See `docs/SECURITY.md` for the full disclosure policy

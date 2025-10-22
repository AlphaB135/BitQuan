<p align="right">
	<a href="./README.th.md"><img alt="ภาษาไทย" src="https://img.shields.io/badge/ภาษาไทย-blue?style=for-the-badge"></a>
</p>

# BitQuan Project Overview

BitQuan targets a 50+ year security horizon with full Post-Quantum Cryptography (PQC) integration. This README provides the English overview by default; click the Thai badge above to switch to the Thai version (`README.th.md`).

## Highlights
- Phase 0 policy: absolutely no backdoors, admin keys, or hidden switches
- Baseline architecture: Minimalist Proof-of-Work with Dilithium signatures (see `docs/architecture/overview.md`)
- Standard documentation set (Governance, Contributing, Release, Security, Reproducibility) under `docs/`

## Repository Map
- `docs/` – Canonical documentation covering governance, security, reproducibility, release process, etc.
- `docs/architecture/overview.md` – Bilingual architecture overview using collapsible language sections
- `docs/security/` – GPG keys, on-call roster, and incident post-mortems
- `todo.md` – Phase-by-phase master plan (Phase 0–13)

## Current Focus
1. Draft transaction and block data specifications (Phase 3)
2. Author BQIP drafts 0001–0004 aligned with the architectural decisions
3. Bootstrap the Rust baseline for core modules: `crypto/`, `consensus/`, `mempool/`, `p2p/`, `storage/`

## Contributing Workflow
- Review `docs/CONTRIBUTING.md` for the code review process and project standards
- Configure deterministic builds per `docs/REPRODUCIBILITY.md`
- Submit signed commits (`git commit -S`) with every pull request

## Security Contact
- Email: `security@bitquan.org`
- Disclosure and bounty policy in `docs/SECURITY.md`

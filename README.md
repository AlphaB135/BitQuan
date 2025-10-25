<p align="right">
	<a href="./README.th.md"><img alt="ภาษาไทย" src="https://img.shields.io/badge/ภาษาไทย-blue?style=for-the-badge"></a>
</p>

# BitQuan

[![CI](https://github.com/alphab/BitQuan/actions/workflows/ci.yml/badge.svg)](https://github.com/alphab/BitQuan/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

BitQuan: PQC-first blockchain (PoW + Dilithium) aiming for 50+ years of security resilience.

This README provides the English overview by default; click the Thai badge above to switch to the Thai version (`README.th.md`).

## Quickstart

```bash
cargo test -p bq-crypto
```

## Documentation
- [Architecture overview](docs/architecture/overview.md)
- [Repository governance](docs/GOVERNANCE.md)
- [Security policy](SECURITY.md)
- [Release process](RELEASE.md)

## Suggested Topics
- Post-Quantum Cryptography (Dilithium, HKDF, hybrid RNGs)
- Proof-of-Work consensus design
- Rust networking, storage, and reproducible builds
- Open governance and long-term security programs

## Security Contact
- Email: [security@bitquan.org](mailto:security@bitquan.org)
- See [`SECURITY.md`](SECURITY.md) for reporting guidelines.

---

## Project Overview

BitQuan targets a 50+ year security horizon with full Post-Quantum Cryptography (PQC) integration.

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

## Additional Security Resources
See the [security policy](SECURITY.md) for disclosure guidelines and contact information.

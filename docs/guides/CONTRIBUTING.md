# Contributing to BitQuan

Thank you for your interest in BitQuan. This guide explains how to contribute code, documentation, testing, and research while upholding the project's security-first principles.

## Core Expectations
- All contributions occur in public repositories with signed commits (`git commit -S`)
- No hidden functionality, privileged keys, or undisclosed network behavior is permitted
- Every change must include tests or benchmarks when feasible, especially for consensus logic
- Follow the BitQuan Improvement Process (BQIP) for protocol-affecting proposals

## Getting Started
1. Fork the repository and create a feature branch from `main`
2. Ensure your environment matches the reproducible build toolchain (see `docs/REPRODUCIBILITY.md`)
3. (Optional) Enable local git hooks for tooling: `./scripts/install-hooks.sh`
4. Run the full test suite and relevant benchmarks before opening a pull request
5. Fill in the pull request template, referencing any related issues or BQIPs

## Code Review Process
- A minimum of two Core Maintainer approvals is required before merge
- Security-sensitive paths (crypto, consensus, networking) need an accompanying design note or threat model update
- Pull requests must include reproducibility metadata: compiler versions, target triples, and deterministic build flags
- Reviewers may request refactoring for clarity, test coverage, or to remove dead code

## Coding Standards
- Prefer Rust for core node implementation; maintainers must approve deviations
- Use `cargo fmt`, `cargo clippy --all-targets --all-features`, and `cargo test --all` before submitting
- Avoid unsafe code unless absolutely necessary; document rationale and mitigation whenever `unsafe` is used
- Document public APIs and security-sensitive modules with concise comments

## Reporting Issues
- Security vulnerabilities: follow the process in `docs/SECURITY.md`
- Consensus bugs or network failures: open a public issue and notify maintainers on the incident channel
- Feature requests: create an issue tagged with `enhancement` and reference supporting BQIP drafts if applicable

## BQIP Workflow
- Drafts live in the `bqip/` directory with incrementing identifiers
- Each BQIP includes motivation, specification, security and economic analyses, and deployment plan
- Adoption requires community review, Steering Committee ratification, and documented activation criteria

## Developer Certificate of Origin
By contributing, you certify that:
- You have the right to submit the contribution
- The contribution complies with the "no backdoor" policy and contains no hidden access mechanisms
- You acknowledge contributions are released under the project's license(s)

Thank you for helping build a secure, transparent, and verifiable post-quantum network.
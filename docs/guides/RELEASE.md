# BitQuan Release Playbook

This document standardizes how BitQuan publishes releases, verifies artifacts, and communicates changes.

## Release Types
- **Mainnet Releases**: Production builds signed and reproducible, versioned `vX.Y.Z`
- **Testnet Releases**: Experimental builds for public testing with clear upgrade notes
- **Security Hotfixes**: Emergency patches with limited scope and mandatory post-mortem

## Pre-Release Checklist
- [ ] All merged pull requests include passing CI (tests, linting, fuzzing)
- [ ] Reproducibility pipeline verified by ≥2 independent builders
- [ ] Release notes drafted with upgrade steps, consensus changes, and incompatibilities
- [ ] Security review sign-off covering cryptography, consensus, networking, and wallets
- [ ] Version numbers bumped consistently in code, documentation, and packaging scripts

## Artifact Production
1. Set `SOURCE_DATE_EPOCH` to the finalized timestamp
2. Build deterministic binaries using the pinned toolchain described in `docs/REPRODUCIBILITY.md`
3. Capture build logs, Git commit hash, and dependency digests
4. Produce signed checksums (`sha256sum` and `sha512sum`) and store in `releases/`
5. Tag the release using signed, annotated tags: `git tag -s vX.Y.Z`

## Verification
- Independent builders reproduce artifacts and compare checksums before public announcement
- Publish verification reports summarizing toolchain versions and diff results
- Distribute GPG public keys for release signers via multiple channels (repository, web, key servers)

## Communication
- Post release announcement on the project website, mailing list, and community channels
- Include upgrade instructions, compatibility warnings, and timeline for mandatory network activation (if any)
- Archive release notes and verification reports in `docs/releases/vX.Y.Z/`

## Post-Release
- Monitor network metrics (orphan rate, latency, error logs) for at least one epoch
- Track bug reports and prioritize patches in the next maintenance release
- Review feedback to update future roadmap items and BQIPs

## Deprecation Policy
- Support the two most recent mainnet release lines with security updates
- Announce deprecations at least one release cycle in advance
- Provide migration tooling and documentation when removing features or changing formats
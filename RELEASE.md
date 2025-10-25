# Release Process

This document summarizes the baseline process for BitQuan releases. A detailed checklist lives in [`docs/RELEASE.md`](docs/RELEASE.md).

## Prerequisites
- All CI pipelines green on `main`.
- Security review completed for new components.
- Update `CHANGELOG.md` with release notes.

## Steps
1. Create a release branch `release/vX.Y.Z`.
2. Update version numbers across crates and documentation.
3. Finalize changelog entries for the release.
4. Run the full reproducible build pipeline described in `docs/REPRODUCIBILITY.md`.
5. Tag the release (`git tag -s vX.Y.Z`) and push tags.
6. Publish signed binaries, checksums, and release notes.

## Post-Release
- Monitor telemetry (opt-in) and community channels.
- Triage follow-up issues and plan hotfixes if required.

This workflow will evolve as we get closer to mainnet; please propose improvements via pull requests.

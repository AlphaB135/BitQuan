# BitQuan Reproducible Builds Guide

Reproducible builds guarantee that anyone can rebuild BitQuan binaries and obtain identical artifacts, reinforcing the "no backdoor" policy.

## Toolchain
- Primary language: Rust (stable channel pinned per release)
- Build system: `cargo` with `CARGO_HOME` and `RUSTUP_HOME` set to project-specific directories
- Deterministic linker: `lld` (or platform equivalent)
- Dependencies vendored in `third_party/` with hash-locked manifests

## Environment Requirements
- Clean, isolated build environment (e.g., Nix shell, Docker image, or reproducible VM)
- Environment variables:
  - `SOURCE_DATE_EPOCH`: canonical release timestamp in seconds
  - `TZ=UTC`
  - `LC_ALL=C`
- Disable network access during build to prevent dependency drift

## Build Steps
1. Checkout the signed release tag: `git clone --branch vX.Y.Z --depth 1`
2. Verify GPG tag signature and commit integrity
3. Restore vendor dependencies: `cargo vendor --locked`
4. Run `cargo build --release --locked --offline -Z unstable-options --config build.rustflags=["-C", "link-arg=-Wl,--build-id=none"]`
5. Strip symbols deterministically using the platform-specific tool (`llvm-strip` suggested)
6. Package artifacts with deterministic archives: `tar --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0`

## Verification
- Produce checksums: `sha256sum <artifact> > artifacts.sha256`
- Compare against published checksum bundle and ensure byte-for-byte match
- Record build metadata: host OS, kernel, compiler hash, `cargo tree --locked` digest

## Independent Builders
- At least two independent parties publish verification reports per release
- Reports include command transcripts, environment manifests, and diff statistics
- Discrepancies trigger a release hold until resolved and documented

## Automation
- CI pipelines replicate the deterministic build using containerized environments
- Nightly builds record digests even if they are not publicly released
- Regression tests flag any drift in build outputs as blocking failures

## Future Enhancements
- Explore in-toto attestations for end-to-end supply-chain integrity
- Integrate reproducibility checks into package managers and node auto-updaters
- Provide optional deterministic builds for ARM, RISC-V, and other architectures
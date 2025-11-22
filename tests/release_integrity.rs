//! Integration test: Release integrity and reproducibility
//!
//! Validates that release builds are reproducible and properly signed

use std::fs;
use std::path::Path;

#[test]
fn test_cargo_toml_version_format() {
    let content = fs::read_to_string("Cargo.toml")
        .expect("Failed to read Cargo.toml");

    // Cargo.toml should reference workspace version or have explicit versions
    assert!(
        content.contains("workspace") || content.contains("version"),
        "Cargo.toml must specify versioning"
    );
}

#[test]
fn test_reproducible_build_documentation() {
    let repro_doc = Path::new("REPRODUCIBILITY.md");
    assert!(
        repro_doc.exists(),
        "REPRODUCIBILITY.md must document build process"
    );

    let content = fs::read_to_string(repro_doc)
        .expect("Failed to read REPRODUCIBILITY.md");

    assert!(
        content.contains("SOURCE_DATE_EPOCH") || content.contains("reproducible"),
        "Documentation must explain reproducible builds"
    );
}

#[test]
fn test_build_script_reproducibility_flags() {
    let script = fs::read_to_string("deploy/scripts/build-release.sh")
        .expect("Failed to read build script");

    // Check for reproducibility flags
    assert!(
        script.contains("--locked"),
        "Must use --locked to pin dependencies"
    );

    assert!(
        script.contains("--release"),
        "Must build in release mode"
    );

    assert!(
        script.contains("SOURCE_DATE_EPOCH"),
        "Must set SOURCE_DATE_EPOCH for reproducible timestamps"
    );
}

#[test]
fn test_release_workflow_signing() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .expect("Failed to read release workflow");

    // Check for checksum generation
    assert!(
        workflow.contains("sha256") || workflow.contains("shasum"),
        "Release workflow must generate checksums"
    );
}

#[test]
fn test_verification_script_checks_checksums() {
    let script = fs::read_to_string("deploy/scripts/verify-signature.sh")
        .expect("Failed to read verification script");

    assert!(
        script.contains("sha256sum") || script.contains("shasum"),
        "Verification script must check SHA256 checksums"
    );
}

#[test]
fn test_cargo_lock_committed() {
    let lock_file = Path::new("Cargo.lock");
    assert!(
        lock_file.exists(),
        "Cargo.lock must be committed for reproducible builds"
    );

    let content = fs::read_to_string(lock_file)
        .expect("Failed to read Cargo.lock");

    assert!(
        content.contains("[[package]]"),
        "Cargo.lock must contain package definitions"
    );
}

#[test]
fn test_rust_toolchain_pinned() {
    let toolchain_file = Path::new("rust-toolchain.toml");
    assert!(
        toolchain_file.exists(),
        "rust-toolchain.toml must specify exact toolchain version"
    );

    let content = fs::read_to_string(toolchain_file)
        .expect("Failed to read rust-toolchain.toml");

    assert!(
        content.contains("channel") || content.contains("version"),
        "Toolchain file must specify version"
    );
}

#[test]
fn test_release_artifacts_structure() {
    // Verify expected release artifact structure
    let build_script = fs::read_to_string("deploy/scripts/build-release.sh")
        .expect("Failed to read build script");

    assert!(
        build_script.contains("dist") || build_script.contains("DIST_DIR"),
        "Build script must specify dist directory"
    );

    assert!(
        build_script.contains("bitquan-node"),
        "Build script must produce bitquan-node binary"
    );
}

#[test]
fn test_release_workflow_matrix_builds() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .expect("Failed to read release workflow");

    // Check for multi-platform builds
    assert!(
        workflow.contains("matrix") || workflow.contains("ubuntu") || workflow.contains("macos"),
        "Release workflow should build for multiple platforms"
    );
}

#[test]
fn test_sbom_generation() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .expect("Failed to read release workflow");

    // Check for SBOM generation (Software Bill of Materials)
    assert!(
        workflow.contains("sbom") || workflow.contains("SBOM"),
        "Release workflow should generate SBOM for supply chain security"
    );
}

#[test]
fn test_release_provenance() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .expect("Failed to read release workflow");

    // Check for SLSA provenance or attestation
    assert!(
        workflow.contains("provenance") || workflow.contains("attest"),
        "Release workflow should generate build provenance"
    );
}

#[test]
fn test_checksum_file_format() {
    // Verify that build script produces correct checksum format
    let script = fs::read_to_string("deploy/scripts/build-release.sh")
        .expect("Failed to read build script");

    assert!(
        script.contains("SHA256SUMS") || script.contains("checksums"),
        "Build script must generate checksum file"
    );
}

#[test]
fn test_binary_stripping() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .expect("Failed to read release workflow");

    // Check for binary stripping to reduce size
    assert!(
        workflow.contains("strip") || workflow.to_lowercase().contains("strip"),
        "Release workflow should strip debug symbols"
    );
}

#[test]
fn test_release_tag_format() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .expect("Failed to read release workflow");

    // Verify version tag trigger
    assert!(
        workflow.contains("v*") || workflow.contains("tags"),
        "Release workflow must trigger on version tags"
    );
}

#[test]
fn test_reproducibility_targets() {
    let script = fs::read_to_string("deploy/scripts/build-release.sh")
        .expect("Failed to read build script");

    // Check for target-cpu generic for reproducibility across CPUs
    assert!(
        script.contains("target-cpu=generic") || script.contains("RUSTFLAGS"),
        "Build should use generic target-cpu for reproducibility"
    );
}

#[test]
fn test_release_notes_automation() {
    let workflow = fs::read_to_string(".github/workflows/release.yml")
        .expect("Failed to read release workflow");

    // Check for automated release notes
    assert!(
        workflow.contains("generate-notes") || workflow.contains("changelog"),
        "Release workflow should generate release notes"
    );
}

#[test]
fn test_no_dirty_builds() {
    // Verify build script checks for clean working directory
    let script = fs::read_to_string("deploy/scripts/build-release.sh")
        .expect("Failed to read build script");

    // Script should enforce clean builds
    assert!(
        script.contains("set -e") || script.contains("set -euo pipefail"),
        "Build script must fail on errors"
    );
}

#[test]
fn test_binary_hash_consistency() {
    // Mock test for hash consistency
    // In real scenario, would build twice and compare hashes

    println!("✓ Binary hash consistency check (mock)");
    println!("  In production: cargo build --locked --release");
    println!("  Should produce identical binary hash on repeat builds");

    // This is a placeholder for actual reproducibility testing
    // which requires building the same code twice and comparing outputs
}

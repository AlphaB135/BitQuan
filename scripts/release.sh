#!/bin/bash
# BitQuan Reproducible Build & Release Script
# Usage: ./scripts/release.sh [version]
# Example: ./scripts/release.sh v1.0.0-rc1

set -euo pipefail

# Configuration
PROJECT_NAME="bitquan"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="$REPO_ROOT/target/release"
DIST_DIR="$REPO_ROOT/dist"
GPG_KEY="${GPG_KEY:-}"  # Set via environment or leave empty for default key

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check for required tools
    local missing_tools=()

    for tool in cargo rustc git gpg sha256sum tar; do
        if ! command -v "$tool" &> /dev/null; then
            missing_tools+=("$tool")
        fi
    done

    if [ ${#missing_tools[@]} -ne 0 ]; then
        log_error "Missing required tools: ${missing_tools[*]}"
        exit 1
    fi

    # Check for GPG key
    if [ -z "$GPG_KEY" ]; then
        log_warn "GPG_KEY not set, will use default signing key"
        if ! gpg --list-secret-keys &> /dev/null; then
            log_error "No GPG keys found. Generate one with: gpg --gen-key"
            exit 1
        fi
    else
        if ! gpg --list-secret-keys "$GPG_KEY" &> /dev/null; then
            log_error "GPG key $GPG_KEY not found"
            exit 1
        fi
    fi

    log_info "Prerequisites OK"
}

# Get version from argument or git tag
get_version() {
    if [ $# -ge 1 ]; then
        VERSION="$1"
    else
        VERSION=$(git describe --tags --abbrev=0 2>/dev/null || echo "v0.0.0-dev")
    fi

    log_info "Building version: $VERSION"
}

# Collect build info
collect_build_info() {
    log_info "Collecting build information..."

    RUSTC_VERSION=$(rustc --version)
    CARGO_VERSION=$(cargo --version)
    GIT_COMMIT=$(git rev-parse HEAD)
    GIT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
    BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    TARGET_TRIPLE=$(rustc -vV | sed -n 's|host: ||p')

    cat > "$DIST_DIR/build-info.txt" <<EOF
BitQuan Build Information
=========================

Version:        $VERSION
Git Commit:     $GIT_COMMIT
Git Branch:     $GIT_BRANCH
Build Date:     $BUILD_DATE
Rust Version:   $RUSTC_VERSION
Cargo Version:  $CARGO_VERSION
Target Triple:  $TARGET_TRIPLE

Reproducibility
---------------
To reproduce this build:
1. Clone repository: git clone https://github.com/AlphaB135/BitQuan.git
2. Checkout commit: git checkout $GIT_COMMIT
3. Build: cargo build --locked --release
4. Verify checksums against SHA256SUMS

Build Environment
-----------------
Operating System: $(uname -s)
Kernel Version:   $(uname -r)
Architecture:     $(uname -m)
EOF

    log_info "Build info saved to $DIST_DIR/build-info.txt"
}

# Clean previous builds
clean_build() {
    log_info "Cleaning previous builds..."

    rm -rf "$DIST_DIR"
    mkdir -p "$DIST_DIR"

    cargo clean --release
}

# Build binaries
build_binaries() {
    log_info "Building binaries with --locked (deterministic dependencies)..."

    # Ensure Cargo.lock is present
    if [ ! -f "$REPO_ROOT/Cargo.lock" ]; then
        log_error "Cargo.lock not found. Run 'cargo build' first to generate it."
        exit 1
    fi

    # Build in release mode with locked dependencies
    cd "$REPO_ROOT"
    cargo build --locked --release

    # Verify binaries exist
    local binaries=(
        "bitquan-node"
        "bitquan-cli"
        "bitquan-wallet"
    )

    for binary in "${binaries[@]}"; do
        if [ ! -f "$BUILD_DIR/$binary" ]; then
            log_warn "Binary not found: $binary (might be optional)"
        else
            log_info "Built: $binary"
        fi
    done
}

# Strip binaries (optional, reduces size but may affect reproducibility)
strip_binaries() {
    log_info "Stripping debug symbols..."

    for binary in "$BUILD_DIR"/*; do
        if [ -x "$binary" ] && [ -f "$binary" ]; then
            strip "$binary" 2>/dev/null || log_warn "Failed to strip $(basename "$binary")"
        fi
    done
}

# Copy binaries to dist
copy_binaries() {
    log_info "Copying binaries to dist directory..."

    for binary in "$BUILD_DIR"/*; do
        if [ -x "$binary" ] && [ -f "$binary" ]; then
            cp "$binary" "$DIST_DIR/"
            log_info "Copied: $(basename "$binary")"
        fi
    done
}

# Generate checksums
generate_checksums() {
    log_info "Generating SHA256 checksums..."

    cd "$DIST_DIR"

    # Create checksums for all binaries
    sha256sum bitquan-* > SHA256SUMS 2>/dev/null || {
        log_error "Failed to generate checksums"
        exit 1
    }

    log_info "Checksums saved to SHA256SUMS"
    cat SHA256SUMS
}

# Sign checksums with GPG
sign_checksums() {
    log_info "Signing checksums with GPG..."

    cd "$DIST_DIR"

    # Remove old signature if exists
    rm -f SHA256SUMS.asc

    # Sign checksums
    if [ -z "$GPG_KEY" ]; then
        gpg --clearsign --armor --output SHA256SUMS.asc SHA256SUMS
    else
        gpg --default-key "$GPG_KEY" --clearsign --armor --output SHA256SUMS.asc SHA256SUMS
    fi

    if [ ! -f SHA256SUMS.asc ]; then
        log_error "Failed to sign checksums"
        exit 1
    fi

    log_info "Signed checksums saved to SHA256SUMS.asc"

    # Verify signature
    gpg --verify SHA256SUMS.asc &> /dev/null && log_info "Signature verified OK" || {
        log_error "Signature verification failed"
        exit 1
    }
}

# Create release archive
create_archive() {
    log_info "Creating release archive..."

    cd "$REPO_ROOT"

    ARCHIVE_NAME="${PROJECT_NAME}-${VERSION}-${TARGET_TRIPLE}"
    ARCHIVE_PATH="$DIST_DIR/${ARCHIVE_NAME}.tar.gz"

    # Create archive with binaries, checksums, and build info
    tar -czf "$ARCHIVE_PATH" \
        -C "$DIST_DIR" \
        --transform "s|^|${ARCHIVE_NAME}/|" \
        bitquan-* \
        SHA256SUMS \
        SHA256SUMS.asc \
        build-info.txt \
        2>/dev/null || {
        log_error "Failed to create archive"
        exit 1
    }

    log_info "Archive created: $ARCHIVE_PATH"

    # Checksum the archive itself
    cd "$DIST_DIR"
    sha256sum "${ARCHIVE_NAME}.tar.gz" > "${ARCHIVE_NAME}.tar.gz.sha256"

    log_info "Archive checksum: ${ARCHIVE_NAME}.tar.gz.sha256"
}

# Display GPG key info
display_key_info() {
    log_info "GPG Key Information:"

    if [ -z "$GPG_KEY" ]; then
        gpg --list-secret-keys | grep -A 1 "sec" | head -2
    else
        gpg --list-secret-keys "$GPG_KEY" | grep -A 1 "sec" | head -2
    fi
}

# Generate release notes template
generate_release_notes() {
    log_info "Generating release notes template..."

    cat > "$DIST_DIR/RELEASE_NOTES.md" <<EOF
# BitQuan $VERSION Release Notes

**Release Date**: $BUILD_DATE
**Git Commit**: $GIT_COMMIT

## Highlights

- [Add key features]
- [Security improvements]
- [Bug fixes]

## Changes Since Last Release

\`\`\`
$(git log --oneline $(git describe --tags --abbrev=0 HEAD^)..HEAD)
\`\`\`

## Verification

### Download

\`\`\`bash
wget https://github.com/AlphaB135/BitQuan/releases/download/$VERSION/${PROJECT_NAME}-${VERSION}-${TARGET_TRIPLE}.tar.gz
\`\`\`

### Verify Checksum

\`\`\`bash
sha256sum -c ${PROJECT_NAME}-${VERSION}-${TARGET_TRIPLE}.tar.gz.sha256
\`\`\`

### Verify GPG Signature

\`\`\`bash
# Import public key (first time only)
gpg --keyserver keyserver.ubuntu.com --recv-keys [KEY_ID]

# Verify signature
gpg --verify SHA256SUMS.asc
\`\`\`

## Installation

\`\`\`bash
tar -xzf ${PROJECT_NAME}-${VERSION}-${TARGET_TRIPLE}.tar.gz
cd ${PROJECT_NAME}-${VERSION}-${TARGET_TRIPLE}
sudo install -m 0755 bitquan-* /usr/local/bin/
\`\`\`

## Known Issues

- [List any known issues]

## Contributors

Thanks to all contributors to this release!

## Full Changelog

See: https://github.com/AlphaB135/BitQuan/compare/[PREVIOUS_TAG]...$VERSION
EOF

    log_info "Release notes template saved to $DIST_DIR/RELEASE_NOTES.md"
}

# Summary
print_summary() {
    echo ""
    echo "=========================================="
    echo "  BitQuan Release Build Complete! 🎉"
    echo "=========================================="
    echo ""
    echo "Version:      $VERSION"
    echo "Commit:       $GIT_COMMIT"
    echo "Target:       $TARGET_TRIPLE"
    echo "Build Date:   $BUILD_DATE"
    echo ""
    echo "Artifacts:"
    echo "  Directory:  $DIST_DIR"
    echo ""
    ls -lh "$DIST_DIR"
    echo ""
    echo "Next Steps:"
    echo "  1. Review release notes: $DIST_DIR/RELEASE_NOTES.md"
    echo "  2. Test binaries: $DIST_DIR/bitquan-*"
    echo "  3. Create GitHub release: gh release create $VERSION"
    echo "  4. Upload artifacts: gh release upload $VERSION $DIST_DIR/*"
    echo ""
    echo "Verification:"
    echo "  sha256sum -c $DIST_DIR/SHA256SUMS"
    echo "  gpg --verify $DIST_DIR/SHA256SUMS.asc"
    echo ""
}

# Main execution
main() {
    log_info "Starting BitQuan reproducible build..."

    get_version "$@"
    check_prerequisites
    clean_build
    collect_build_info
    build_binaries
    # strip_binaries  # Optional: uncomment to reduce binary size
    copy_binaries
    generate_checksums
    sign_checksums
    create_archive
    generate_release_notes
    display_key_info
    print_summary

    log_info "Release build completed successfully!"
}

# Run main function
main "$@"

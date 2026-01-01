#!/usr/bin/env bash
#----------------------------------------------------------
# Shebe CI Build Script
#
# Builds release binaries and creates distribution tarballs.
# Extracts version from Cargo.toml.
#
# Usage:
#   ./scripts/ci-build.sh
#
# Environment variables:
#   CI_PROJECT_DIR    - Repository root (GitLab CI predefined, auto-detected locally)
#   SHEBE_SERVICE_DIR - Path to shebe-server (default: services/shebe-server)
#   RELEASE_DIR       - Output directory (default: releases)
#
# Outputs:
#   releases/shebe-v{VERSION}-linux-x86_64.tar.gz
#   releases/shebe-v{VERSION}-linux-x86_64.tar.gz.sha256
#----------------------------------------------------------
set -euo pipefail

# Use CI_PROJECT_DIR in GitLab CI, otherwise calculate from script location
if [[ -n "${CI_PROJECT_DIR:-}" ]]; then
    REPO_ROOT="${CI_PROJECT_DIR}"
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
fi

# Configuration
SHEBE_SERVICE_DIR="${SHEBE_SERVICE_DIR:-services/shebe-server}"
RELEASE_DIR="${RELEASE_DIR:-releases}"
CARGO_TOML="${REPO_ROOT}/${SHEBE_SERVICE_DIR}/Cargo.toml"

#----------------------------------------------------------
# Functions
#----------------------------------------------------------

log() {
    echo "[ci-build] $*"
}

error() {
    echo "[ci-build] ERROR: $*" >&2
    exit 1
}

extract_version() {
    if [[ ! -f "${CARGO_TOML}" ]]; then
        error "Cargo.toml not found at ${CARGO_TOML}"
    fi
    grep '^version' "${CARGO_TOML}" | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

#----------------------------------------------------------
# Main
#----------------------------------------------------------

main() {
    log "Starting build process"
    log "Repository root: ${REPO_ROOT}"

    # Extract version
    VERSION=$(extract_version)
    if [[ -z "${VERSION}" ]]; then
        error "Failed to extract version from Cargo.toml"
    fi
    log "Version: ${VERSION}"

    # Change to service directory
    cd "${REPO_ROOT}/${SHEBE_SERVICE_DIR}"
    log "Working directory: $(pwd)"

    # Display toolchain info
    log "Rust toolchain:"
    rustc --version
    cargo --version

    # Build release binaries to build/ directory (matches Makefile convention)
    log "Building release binaries..."
    cargo build --release --target-dir build

    # Verify binaries exist
    local target_dir="build/release"
    local binaries=("shebe" "shebe-mcp")
    for binary in "${binaries[@]}"; do
        if [[ ! -f "${target_dir}/${binary}" ]]; then
            error "Binary not found: ${target_dir}/${binary}"
        fi
        log "Built: ${binary} ($(du -h "${target_dir}/${binary}" | cut -f1))"
    done

    # Create release directory
    local release_path="${REPO_ROOT}/${RELEASE_DIR}"
    mkdir -p "${release_path}"

    # Define artifact names
    local tarball_name="shebe-v${VERSION}-linux-x86_64.tar.gz"
    local tarball_path="${release_path}/${tarball_name}"
    local checksum_path="${tarball_path}.sha256"

    # Create tarball
    log "Creating tarball: ${tarball_name}"
    tar -czf "${tarball_path}" \
        -C "${target_dir}" \
        "${binaries[@]}"

    # Generate checksum
    log "Generating SHA256 checksum"
    cd "${release_path}"
    sha256sum "${tarball_name}" > "${tarball_name}.sha256"

    # Display results
    log "Build complete:"
    ls -la "${release_path}/"

    log "Checksum:"
    cat "${checksum_path}"

    # Export version for downstream jobs
    if [[ -n "${CI:-}" ]]; then
        echo "VERSION=${VERSION}" >> "${REPO_ROOT}/build.env"
        log "Exported VERSION=${VERSION} to build.env"
    fi

    log "Build successful"
}

main "$@"

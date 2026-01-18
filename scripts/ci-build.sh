#!/usr/bin/env bash
#----------------------------------------------------------
# Shebe CI Build Script
#
# Builds release binaries and creates distribution tarballs.
# Supports both glibc (dynamic) and musl (static) targets.
# Extracts version from Cargo.toml.
#
# Usage:
#   ./scripts/ci-build.sh
#
# Environment variables:
#   CI_PROJECT_DIR    - Repository root (GitLab CI predefined, auto-detected locally)
#   SHEBE_SERVICE_DIR - Path to shebe-server (default: services/shebe-server)
#   RELEASE_DIR       - Output directory (default: releases)
#   BUILD_TARGET      - Rust target (default: builds both glibc and musl)
#   ARTIFACT_SUFFIX   - Tarball suffix (default: derived from target)
#
# Outputs (single target mode - CI matrix):
#   releases/shebe-v{VERSION}-{ARTIFACT_SUFFIX}.tar.gz
#   releases/shebe-v{VERSION}-{ARTIFACT_SUFFIX}.tar.gz.sha256
#
# Outputs (all targets mode - local):
#   releases/shebe-v{VERSION}-linux-x86_64.tar.gz           (glibc, dynamic)
#   releases/shebe-v{VERSION}-linux-x86_64.tar.gz.sha256
#   releases/shebe-v{VERSION}-linux-x86_64-musl.tar.gz      (musl, static)
#   releases/shebe-v{VERSION}-linux-x86_64-musl.tar.gz.sha256
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

# Binaries to include in tarballs
BINARIES=("shebe" "shebe-mcp")

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

# Build for a specific target and create tarball
build_target() {
    local target="$1"
    local suffix="$2"
    local version="$3"
    local release_path="$4"

    log "Building for target: ${target}"

    if [[ "${target}" == *"musl"* ]]; then
        # Static build with musl - ensure fully static linking
        log "Using static linking (musl)"
        RUSTFLAGS="-C target-feature=+crt-static" \
            cargo build --release --target "${target}" --target-dir build
    else
        # Standard glibc build (dynamic linking)
        log "Using dynamic linking (glibc)"
        cargo build --release --target "${target}" --target-dir build
    fi

    # Target directory contains the built binaries
    local target_dir="build/${target}/release"

    # Verify binaries exist
    for binary in "${BINARIES[@]}"; do
        if [[ ! -f "${target_dir}/${binary}" ]]; then
            error "Binary not found: ${target_dir}/${binary}"
        fi
        log "Built: ${binary} ($(du -h "${target_dir}/${binary}" | cut -f1))"
    done

    # Show linking info for verification
    log "Checking linking for ${target}:"
    if command -v ldd &> /dev/null; then
        ldd "${target_dir}/${BINARIES[0]}" 2>&1 || log "  (static binary - no dynamic dependencies)"
    fi

    # Define artifact names
    local tarball_name="shebe-v${version}-${suffix}.tar.gz"

    # Create tarball
    log "Creating tarball: ${tarball_name}"
    tar -czf "${release_path}/${tarball_name}" \
        -C "${target_dir}" \
        "${BINARIES[@]}"

    # Generate checksum
    log "Generating SHA256 checksum for ${tarball_name}"
    (cd "${release_path}" && sha256sum "${tarball_name}" > "${tarball_name}.sha256")

    log "Completed build for ${target}"
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

    # Show installed targets
    log "Installed targets:"
    rustup target list --installed

    # Create release directory
    local release_path="${REPO_ROOT}/${RELEASE_DIR}"
    mkdir -p "${release_path}"

    # Check if running in matrix mode (single target) or local mode (all targets)
    if [[ -n "${BUILD_TARGET:-}" && -n "${ARTIFACT_SUFFIX:-}" ]]; then
        # Matrix mode: build single target from environment variables
        log "Matrix mode: building single target"
        build_target "${BUILD_TARGET}" "${ARTIFACT_SUFFIX}" "${VERSION}" "${release_path}"
    else
        # Local mode: build all targets sequentially
        log "Local mode: building all targets"
        local targets=(
            "x86_64-unknown-linux-gnu:linux-x86_64"
            "x86_64-unknown-linux-musl:linux-x86_64-musl"
        )
        for target_spec in "${targets[@]}"; do
            IFS=':' read -r target suffix <<< "${target_spec}"
            build_target "${target}" "${suffix}" "${VERSION}" "${release_path}"
            echo ""  # Visual separation between targets
        done
    fi

    # Display results
    log "Build complete. Artifacts:"
    ls -la "${release_path}/"

    log "Checksums:"
    cat "${release_path}"/*.sha256

    # Export version for downstream jobs
    if [[ -n "${CI:-}" ]]; then
        echo "VERSION=${VERSION}" >> "${REPO_ROOT}/build.env"
        log "Exported VERSION=${VERSION} to build.env"
    fi

    log "Build successful"
}

main "$@"

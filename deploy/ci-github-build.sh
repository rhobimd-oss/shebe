#!/usr/bin/env bash
#----------------------------------------------------------
# Shebe GitHub Actions Build Script (macOS)
#
# Builds release binaries for macOS targets.
# Designed to run in GitHub Actions but testable locally.
#
# Usage:
#   ./deploy/ci-github-build.sh --target aarch64-apple-darwin
#   ./deploy/ci-github-build.sh --target x86_64-apple-darwin --preview
#
# Required:
#   --target TARGET    Rust target triple (e.g., aarch64-apple-darwin)
#
# Optional:
#   --preview          Preview without building
#   --help, -h         Show this help message
#
# Environment variables:
#   GITHUB_OUTPUT      GitHub Actions output file (set automatically in Actions)
#
# Outputs:
#   shebe-v{VERSION}-darwin-{ARCH}.tar.gz
#   shebe-v{VERSION}-darwin-{ARCH}.tar.gz.sha256
#
# GitHub Actions outputs (via $GITHUB_OUTPUT):
#   version            Version with 'v' prefix (e.g., v0.6.0)
#   artifact_name      Full artifact name without extension
#----------------------------------------------------------
set -euo pipefail

# Determine repository root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Configuration
SHEBE_SERVICE_DIR="services/shebe-server"
CARGO_TOML="${REPO_ROOT}/${SHEBE_SERVICE_DIR}/Cargo.toml"
BINARIES=("shebe" "shebe-mcp")

# Arguments
TARGET=""
PREVIEW_MODE=false

#----------------------------------------------------------
# Functions
#----------------------------------------------------------

log() {
    echo "[ci-github-build] $*"
}

error() {
    echo "[ci-github-build] ERROR: $*" >&2
    exit 1
}

usage() {
    echo "Usage: $0 --target TARGET [OPTIONS]"
    echo ""
    echo "Required:"
    echo "  --target TARGET    Rust target triple (e.g., aarch64-apple-darwin)"
    echo ""
    echo "Options:"
    echo "  --preview          Preview build configuration without building"
    echo "  --help, -h         Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --target aarch64-apple-darwin              # Build for Apple Silicon"
    echo "  $0 --target x86_64-apple-darwin               # Build for Intel Mac"
    echo "  $0 --target aarch64-apple-darwin --preview    # Preview only"
    exit 0
}

extract_version() {
    if [[ ! -f "${CARGO_TOML}" ]]; then
        error "Cargo.toml not found at ${CARGO_TOML}"
    fi
    grep '^version' "${CARGO_TOML}" | head -1 | sed 's/.*"\(.*\)".*/\1/'
}

# Extract architecture from target triple
# aarch64-apple-darwin -> aarch64
# x86_64-apple-darwin -> x86_64
get_arch_from_target() {
    local target="$1"
    echo "${target%%-*}"
}

# Set GitHub Actions output
set_output() {
    local name="$1"
    local value="$2"
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        echo "${name}=${value}" >> "${GITHUB_OUTPUT}"
        log "Set output: ${name}=${value}"
    fi
}

build_and_package() {
    local target="$1"
    local version="$2"
    local arch
    arch=$(get_arch_from_target "${target}")
    local artifact_name="shebe-v${version}-darwin-${arch}"

    log "Building for target: ${target}"
    log "Architecture: ${arch}"
    log "Artifact name: ${artifact_name}"

    # Build
    cd "${REPO_ROOT}/${SHEBE_SERVICE_DIR}"
    cargo build --release --target "${target}"

    local target_dir="target/${target}/release"

    # Verify binaries exist
    for binary in "${BINARIES[@]}"; do
        if [[ ! -f "${target_dir}/${binary}" ]]; then
            error "Binary not found: ${target_dir}/${binary}"
        fi
        log "Built: ${binary} ($(du -h "${target_dir}/${binary}" | cut -f1))"
    done

    # Show binary info
    log "Binary info:"
    for binary in "${BINARIES[@]}"; do
        file "${target_dir}/${binary}"
    done

    # Create dist directory and copy binaries
    cd "${REPO_ROOT}"
    mkdir -p dist
    for binary in "${BINARIES[@]}"; do
        cp "${SHEBE_SERVICE_DIR}/${target_dir}/${binary}" dist/
    done

    # Create tarball
    log "Creating tarball: ${artifact_name}.tar.gz"
    cd dist
    tar -czvf "../${artifact_name}.tar.gz" "${BINARIES[@]}"
    cd ..

    # Generate checksum (use shasum on macOS, sha256sum on Linux)
    log "Generating SHA256 checksum"
    if command -v shasum &> /dev/null; then
        shasum -a 256 "${artifact_name}.tar.gz" > "${artifact_name}.tar.gz.sha256"
    else
        sha256sum "${artifact_name}.tar.gz" > "${artifact_name}.tar.gz.sha256"
    fi

    # Display results
    log "Created artifact: ${artifact_name}.tar.gz"
    cat "${artifact_name}.tar.gz.sha256"

    # Cleanup dist directory
    rm -rf dist

    # Set GitHub Actions outputs
    set_output "version" "v${version}"
    set_output "artifact_name" "${artifact_name}"
}

#----------------------------------------------------------
# Main
#----------------------------------------------------------

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --target)
                TARGET="$2"
                shift 2
                ;;
            --preview)
                PREVIEW_MODE=true
                shift
                ;;
            --help|-h)
                usage
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done

    # Validate required arguments
    if [[ -z "${TARGET}" ]]; then
        error "Missing required argument: --target"
    fi

    log "Starting GitHub Actions build"
    log "Repository root: ${REPO_ROOT}"
    log "Target: ${TARGET}"

    # Extract version
    VERSION=$(extract_version)
    if [[ -z "${VERSION}" ]]; then
        error "Failed to extract version from Cargo.toml"
    fi
    log "Version: ${VERSION}"

    # Get architecture
    ARCH=$(get_arch_from_target "${TARGET}")
    ARTIFACT_NAME="shebe-v${VERSION}-darwin-${ARCH}"

    # Preview mode
    if [[ "${PREVIEW_MODE}" == "true" ]]; then
        log "Preview mode - would perform these actions:"
        echo ""
        echo "1. Change to directory: ${REPO_ROOT}/${SHEBE_SERVICE_DIR}"
        echo "2. Run: cargo build --release --target ${TARGET}"
        echo "3. Create tarball: ${ARTIFACT_NAME}.tar.gz"
        echo "   Contents: ${BINARIES[*]}"
        echo "4. Generate checksum: ${ARTIFACT_NAME}.tar.gz.sha256"
        echo "5. Output directory: ${REPO_ROOT}/"
        echo ""
        echo "GitHub Actions outputs:"
        echo "  version=${VERSION}"
        echo "  artifact_name=${ARTIFACT_NAME}"
        echo ""
        if command -v rustc &> /dev/null; then
            log "Rust toolchain:"
            rustc --version
            cargo --version
        else
            log "Rust toolchain: not installed (will be installed by GitHub Actions)"
        fi
        echo ""
        log "Preview complete"
        return 0
    fi

    # Build and package
    build_and_package "${TARGET}" "${VERSION}"

    log "Build successful"
}

main "$@"

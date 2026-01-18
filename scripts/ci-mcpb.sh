#!/usr/bin/env bash
#----------------------------------------------------------
# Shebe CI MCPB Bundle Script
#
# Creates an MCPB (MCP Bundle) from the musl static binary.
# The bundle is a ZIP file containing:
#   - manifest.json (with version updated)
#   - icon.png
#   - server/shebe-mcp (static binary)
#
# Usage:
#   ./scripts/ci-mcpb.sh              # Run in GitLab CI
#   ./scripts/ci-mcpb.sh --preview    # Local preview (no upload)
#
# Required environment variables (GitLab CI predefined):
#   CI_COMMIT_TAG       - Git tag (e.g., v0.5.6)
#   CI_PROJECT_ID       - GitLab project ID
#   CI_API_V4_URL       - GitLab API URL
#   CI_JOB_TOKEN        - Job token for API authentication
#
# Optional:
#   RELEASE_DIR         - Directory containing release artifacts (default: releases)
#----------------------------------------------------------
set -euo pipefail

# Use CI_PROJECT_DIR in GitLab CI, otherwise calculate from script location
if [[ -n "${CI_PROJECT_DIR:-}" ]]; then
    REPO_ROOT="${CI_PROJECT_DIR}"
else
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
fi

RELEASE_DIR="${RELEASE_DIR:-releases}"
PREVIEW_MODE=false

#----------------------------------------------------------
# Functions
#----------------------------------------------------------

log() {
    echo "[ci-mcpb] $*"
}

error() {
    echo "[ci-mcpb] ERROR: $*" >&2
    exit 1
}

# Check required dependencies
check_dependencies() {
    local missing=()
    for cmd in zip unzip sha256sum tar curl; do
        if ! command -v "${cmd}" &> /dev/null; then
            missing+=("${cmd}")
        fi
    done
    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Missing required commands: ${missing[*]}"
    fi
}

# Upload a single file to package registry
upload_file() {
    local url="$1"
    local filepath="$2"
    local filename
    filename=$(basename "${filepath}")

    log "Uploading: ${filename}"

    local response
    response=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
        --header "JOB-TOKEN: ${CI_JOB_TOKEN}" \
        --upload-file "${filepath}" \
        "${url}")

    local http_code
    http_code=$(echo "${response}" | tail -1 | sed 's/.*HTTP_CODE://')

    if [[ "${http_code}" -ne 201 ]]; then
        local response_body
        response_body=$(echo "${response}" | sed '$d')
        error "Failed to upload ${filename} (HTTP ${http_code}): ${response_body}"
    fi
    log "Uploaded: ${filename}"
}

# Generate server.json for MCP Registry publication
# Uses GitLab release asset URL pattern required by MCP Registry:
#   /owner/repo/-/releases/tag/downloads/filename
generate_server_json() {
    local version="$1"
    local sha256="$2"
    # MCP Registry requires release asset URL (not API URL)
    local mcpb_url="https://gitlab.com/rhobimd-oss/shebe/-/releases/v${version}/downloads/shebe-v${version}.mcpb"

    log "Generating server.json for MCP Registry..."

    cat > "${RELEASE_DIR}/server.json" << EOF
{
  "\$schema": "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json",
  "name": "health.rhobimd.oss/shebe",
  "description": "Fast BM25 full-text search for code repositories. 14 MCP tools, 2ms search latency.",
  "version": "${version}",
  "repository": {
    "url": "https://gitlab.com/rhobimd-oss/shebe",
    "source": "gitlab"
  },
  "homepage": "https://gitlab.com/rhobimd-oss/shebe",
  "license": "Apache-2.0",
  "author": {
    "name": "RHOBIMD",
    "url": "https://rhobimd.health"
  },
  "packages": [
    {
      "registryType": "mcpb",
      "identifier": "${mcpb_url}",
      "fileSha256": "${sha256}",
      "transport": {
        "type": "stdio"
      }
    }
  ],
  "categories": ["developer-tools", "search"],
  "keywords": ["bm25", "code-search", "tantivy", "rust", "rag"]
}
EOF

    log "server.json created: ${RELEASE_DIR}/server.json"
}

#----------------------------------------------------------
# Main
#----------------------------------------------------------

main() {
    # Check dependencies first
    check_dependencies

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --preview)
                PREVIEW_MODE=true
                shift
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done

    cd "${REPO_ROOT}"

    # Get version from tag or VERSION file
    local version
    if [[ -n "${CI_COMMIT_TAG:-}" ]]; then
        version="${CI_COMMIT_TAG#v}"
    else
        local version_file="${REPO_ROOT}/services/shebe-server/VERSION"
        if [[ -f "${version_file}" ]]; then
            version="$(cat "${version_file}")"
        else
            error "No CI_COMMIT_TAG or VERSION file found"
        fi
    fi

    log "Creating MCPB bundle for version ${version}"

    local mcpb_name="shebe-v${version}.mcpb"
    local musl_tarball="${RELEASE_DIR}/shebe-v${version}-linux-x86_64-musl.tar.gz"

    # Verify musl tarball exists
    if [[ ! -f "${musl_tarball}" ]]; then
        error "Musl tarball not found: ${musl_tarball}"
    fi

    # Verify mcpb assets exist
    if [[ ! -f "mcpb/manifest.json" ]]; then
        error "manifest.json not found: mcpb/manifest.json"
    fi
    if [[ ! -f "mcpb/icon.png" ]]; then
        error "icon.png not found: mcpb/icon.png"
    fi

    # Create temporary bundle directory
    local bundle_dir
    bundle_dir=$(mktemp -d)
    mkdir -p "${bundle_dir}/server"

    log "Extracting binary from musl tarball..."
    tar -xzf "${musl_tarball}" -C "${bundle_dir}/server" shebe-mcp
    chmod +x "${bundle_dir}/server/shebe-mcp"

    log "Copying manifest and updating version..."
    sed "s/\"version\": \"[^\"]*\"/\"version\": \"${version}\"/" \
        mcpb/manifest.json > "${bundle_dir}/manifest.json"

    log "Copying icon..."
    cp mcpb/icon.png "${bundle_dir}/"

    log "Creating ZIP bundle..."
    mkdir -p "${RELEASE_DIR}"
    (cd "${bundle_dir}" && zip -r - .) > "${RELEASE_DIR}/${mcpb_name}"

    log "Generating checksum..."
    (cd "${RELEASE_DIR}" && sha256sum "${mcpb_name}" > "${mcpb_name}.sha256")

    # Cleanup temp directory
    rm -rf "${bundle_dir}"

    # Extract SHA256 hash (just the hash, not the filename)
    local mcpb_sha256
    mcpb_sha256=$(cut -d' ' -f1 "${RELEASE_DIR}/${mcpb_name}.sha256")

    log "Bundle created: ${RELEASE_DIR}/${mcpb_name}"
    log "SHA256: ${mcpb_sha256}"

    # Verify bundle contents
    log "Bundle contents:"
    unzip -l "${RELEASE_DIR}/${mcpb_name}"

    # Generate server.json for MCP Registry publication
    generate_server_json "${version}" "${mcpb_sha256}"

    if [[ "${PREVIEW_MODE}" == "true" ]]; then
        log "Preview mode - skipping upload"
        log "Files generated:"
        log "  - ${RELEASE_DIR}/${mcpb_name}"
        log "  - ${RELEASE_DIR}/${mcpb_name}.sha256"
        log "  - ${RELEASE_DIR}/server.json"
    else
        # Upload to package registry
        if [[ -z "${CI_JOB_TOKEN:-}" ]]; then
            error "CI_JOB_TOKEN not set - cannot upload"
        fi

        local base_url="${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/shebe/${version}"

        log "Uploading to package registry..."
        upload_file "${base_url}/${mcpb_name}" "${RELEASE_DIR}/${mcpb_name}"
        upload_file "${base_url}/${mcpb_name}.sha256" "${RELEASE_DIR}/${mcpb_name}.sha256"
        upload_file "${base_url}/server.json" "${RELEASE_DIR}/server.json"

        log "MCPB bundle and server.json uploaded successfully"
    fi
}

main "$@"

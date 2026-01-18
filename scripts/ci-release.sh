#!/usr/bin/env bash
#----------------------------------------------------------
# Shebe CI Release Script
#
# Creates a GitLab release with changelog and artifact links.
# Uploads both glibc and musl tarballs to package registry.
# Uses CI_JOB_TOKEN for authentication (no manual token needed).
#
# Usage:
#   ./scripts/ci-release.sh              # Run in GitLab CI
#   ./scripts/ci-release.sh --preview    # Local preview (no API calls)
#   ./scripts/ci-release.sh --preview v0.5.0  # Preview specific version
#
# Required environment variables (GitLab CI predefined):
#   CI_PROJECT_DIR      - Repository root directory
#   CI_COMMIT_TAG       - Git tag (e.g., v0.4.1)
#   CI_PROJECT_URL      - GitLab project URL
#   CI_PROJECT_ID       - GitLab project ID
#   CI_API_V4_URL       - GitLab API URL
#   CI_COMMIT_SHA       - Full commit SHA
#   CI_COMMIT_SHORT_SHA - Short commit SHA
#   CI_JOB_TOKEN        - Job token for API authentication
#
# Optional environment variables:
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

# Configuration
RELEASE_DIR="${RELEASE_DIR:-releases}"
PREVIEW_MODE=false

# Release artifacts: "filename-pattern:description"
# Each pattern is expanded with version
RELEASE_ARTIFACTS=(
    "linux-x86_64:Linux x86_64 (glibc)"
    "linux-x86_64-musl:Linux x86_64 (musl, static)"
)

#----------------------------------------------------------
# Functions
#----------------------------------------------------------

log() {
    echo "[ci-release] $*"
}

error() {
    echo "[ci-release] ERROR: $*" >&2
    exit 1
}

usage() {
    echo "Usage: $0 [--preview [VERSION]]"
    echo ""
    echo "Options:"
    echo "  --preview [VERSION]  Preview release notes locally (no API calls)"
    echo "                       VERSION defaults to next version from VERSION file"
    echo ""
    echo "Examples:"
    echo "  $0                   # Run in GitLab CI (requires CI variables)"
    echo "  $0 --preview         # Preview with version from VERSION file"
    echo "  $0 --preview v0.5.0  # Preview specific version"
    exit 0
}

setup_preview_environment() {
    local version="${1:-}"

    # Get version from VERSION file if not provided
    if [[ -z "${version}" ]]; then
        local version_file="${REPO_ROOT}/services/shebe-server/VERSION"
        if [[ -f "${version_file}" ]]; then
            version="v$(cat "${version_file}")"
        else
            error "No version provided and VERSION file not found"
        fi
    fi

    # Ensure version starts with 'v'
    if [[ "${version}" != v* ]]; then
        version="v${version}"
    fi

    # Set mock CI variables for preview
    export CI_COMMIT_TAG="${version}"
    export CI_PROJECT_URL="https://gitlab.com/rhobimd-oss/shebe"
    export CI_PROJECT_ID="preview"
    export CI_API_V4_URL="https://gitlab.com/api/v4"
    export CI_COMMIT_SHA="$(git rev-parse HEAD 2>/dev/null || echo "preview")"
    export CI_COMMIT_SHORT_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo "preview")"

    log "Preview mode enabled for ${CI_COMMIT_TAG}"
}

validate_environment() {
    log "Validating environment..."

    if [[ "${PREVIEW_MODE}" == "true" ]]; then
        log "Preview mode - skipping CI variable validation"
        return 0
    fi

    if [[ -z "${CI_COMMIT_TAG:-}" ]]; then
        error "CI_COMMIT_TAG is not set. This script should only run on Git tags."
    fi

    if [[ -z "${CI_JOB_TOKEN:-}" ]]; then
        error "CI_JOB_TOKEN is not set. This script must run in a GitLab CI job."
    fi

    local required_vars=(
        "CI_PROJECT_URL"
        "CI_PROJECT_ID"
        "CI_API_V4_URL"
        "CI_COMMIT_SHA"
        "CI_COMMIT_SHORT_SHA"
    )

    for var in "${required_vars[@]}"; do
        if [[ -z "${!var:-}" ]]; then
            error "Required variable ${var} is not set"
        fi
    done

    log "Environment validated"
}

get_previous_tag() {
    git tag --sort=-version:refname | grep -v "^${CI_COMMIT_TAG}$" | head -1 || echo ""
}

# Build package registry URL for a given file
# Uses numeric project ID for reliability
get_package_url() {
    local version="$1"
    local filename="$2"
    echo "${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/packages/generic/shebe/${version}/${filename}"
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

# Upload release artifacts to GitLab Package Registry
# Package registry is public for public projects, unlike job artifacts
upload_to_package_registry() {
    local version="$1"
    local release_path="${REPO_ROOT}/${RELEASE_DIR}"

    log "Uploading to package registry..."

    # Upload each artifact (tarball + checksum)
    for artifact_spec in "${RELEASE_ARTIFACTS[@]}"; do
        IFS=':' read -r suffix description <<< "${artifact_spec}"
        local tarball="shebe-v${version}-${suffix}.tar.gz"
        local checksum="${tarball}.sha256"

        # Upload tarball
        local tarball_url
        tarball_url=$(get_package_url "${version}" "${tarball}")
        upload_file "${tarball_url}" "${release_path}/${tarball}"

        # Upload checksum
        local checksum_url
        checksum_url=$(get_package_url "${version}" "${checksum}")
        upload_file "${checksum_url}" "${release_path}/${checksum}"
    done

    log "Package registry upload complete"
}

# Extract changelog section from CHANGELOG.md for the release.
# Grabs the [Unreleased] section and rewrites the header with the version being released.
# Falls back to git log if [Unreleased] section is empty or not found.
extract_changelog_section() {
    local version="$1"
    local changelog_file="${REPO_ROOT}/CHANGELOG.md"
    local output_file="${REPO_ROOT}/RELEASE_CHANGELOG.md"

    log "Extracting changelog for version ${version}..."

    if [[ -f "${changelog_file}" ]]; then
        # Extract [Unreleased] section (everything between ## [Unreleased] and next ## [)
        local section
        section=$(awk '
            /^## \[Unreleased\]/ { found=1; next }
            /^## \[/ { if (found) exit }
            found { print }
        ' "${changelog_file}")

        # Check if we got meaningful content (not just whitespace)
        if [[ -n "$(echo "${section}" | grep -v '^[[:space:]]*$')" ]]; then
            {
                echo "## [${version}] - $(date -u +"%Y-%m-%d")"
                echo "${section}"
            } > "${output_file}"
            log "Extracted [Unreleased] section from CHANGELOG.md"
            return 0
        fi
    fi

    # Fallback: generate from git log
    log "[Unreleased] section empty or not found, generating from git history..."
    local previous_tag
    previous_tag=$(get_previous_tag)

    if [[ -n "${previous_tag}" ]]; then
        {
            echo "## [${version}] - $(date -u +"%Y-%m-%d")"
            echo ""
            echo "### Changes"
            echo ""
            git log --pretty=format:"- %s ([%h](${CI_PROJECT_URL}/-/commit/%H))" \
                "${previous_tag}..${CI_COMMIT_TAG}" || true
            echo ""
        } > "${output_file}"
    else
        {
            echo "## [${version}] - $(date -u +"%Y-%m-%d")"
            echo ""
            echo "Initial release of Shebe!"
        } > "${output_file}"
    fi
}

generate_release_notes() {
    local version="$1"
    local release_notes_file="${REPO_ROOT}/RELEASE_NOTES.md"
    local release_changelog="${REPO_ROOT}/RELEASE_CHANGELOG.md"

    log "Generating release notes..."

    # Build downloads table rows
    local downloads_table=""
    for artifact_spec in "${RELEASE_ARTIFACTS[@]}"; do
        IFS=':' read -r suffix description <<< "${artifact_spec}"
        local tarball="shebe-v${version}-${suffix}.tar.gz"
        local tarball_url
        local checksum_url
        tarball_url=$(get_package_url "${version}" "${tarball}")
        checksum_url=$(get_package_url "${version}" "${tarball}.sha256")
        downloads_table="${downloads_table}| ${description} | [${tarball}](${tarball_url}) | [SHA256](${checksum_url}) |
"
    done

    # Get URLs for installation example (use glibc version)
    local install_tarball="shebe-v${version}-linux-x86_64.tar.gz"
    local install_url
    install_url=$(get_package_url "${version}" "${install_tarball}")

    cat > "${release_notes_file}" << EOF
# Shebe ${CI_COMMIT_TAG}

**Release Date:** $(date -u +"%Y-%m-%d")
**Commit:** [\`${CI_COMMIT_SHORT_SHA}\`](${CI_PROJECT_URL}/-/commit/${CI_COMMIT_SHA})

## Downloads

| Platform | Download | Checksum |
|----------|----------|----------|
${downloads_table}
**Note:** Use the musl (static) build for MCPB bundles or Alpine-based containers.
Use the glibc build for standard Linux distributions.

## Installation

\`\`\`bash
# Download and extract (glibc version)
curl -LO "${install_url}"
tar -xzf ${install_tarball}

# Move binaries to PATH
sudo mv shebe shebe-mcp /usr/local/bin/
\`\`\`

$(cat "${release_changelog}")

---
[All Releases](${CI_PROJECT_URL}/-/releases) | [Documentation](${CI_PROJECT_URL}/-/blob/main/README.md) | [Full Changelog](${CI_PROJECT_URL}/-/blob/main/CHANGELOG.md)
EOF

    log "Release notes generated: ${release_notes_file}"
}

create_gitlab_release() {
    local version="$1"
    local release_notes_file="${REPO_ROOT}/RELEASE_NOTES.md"

    log "Creating GitLab release..."

    # Build asset links JSON array
    local asset_links="["
    local first=true
    for artifact_spec in "${RELEASE_ARTIFACTS[@]}"; do
        IFS=':' read -r suffix description <<< "${artifact_spec}"
        local tarball="shebe-v${version}-${suffix}.tar.gz"
        local tarball_url
        local checksum_url
        tarball_url=$(get_package_url "${version}" "${tarball}")
        checksum_url=$(get_package_url "${version}" "${tarball}.sha256")

        if [[ "${first}" != "true" ]]; then
            asset_links="${asset_links},"
        fi
        first=false

        # Add tarball link
        asset_links="${asset_links}{\"name\":\"${tarball}\",\"url\":\"${tarball_url}\",\"link_type\":\"package\"}"
        # Add checksum link
        asset_links="${asset_links},{\"name\":\"${tarball}.sha256\",\"url\":\"${checksum_url}\",\"link_type\":\"other\"}"
    done
    asset_links="${asset_links}]"

    # Build release payload
    local payload
    payload=$(jq -n \
        --arg tag "${CI_COMMIT_TAG}" \
        --arg name "Shebe ${CI_COMMIT_TAG}" \
        --arg description "$(cat "${release_notes_file}")" \
        --arg ref "${CI_COMMIT_SHA}" \
        --argjson links "${asset_links}" \
        '{
            tag_name: $tag,
            name: $name,
            description: $description,
            ref: $ref,
            assets: {
                links: $links
            }
        }')

    # Submit release to GitLab API using CI_JOB_TOKEN
    local response
    response=$(curl -s -w "\nHTTP_CODE:%{http_code}" \
        -X POST \
        -H "JOB-TOKEN: ${CI_JOB_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "${payload}" \
        "${CI_API_V4_URL}/projects/${CI_PROJECT_ID}/releases")

    local http_code
    http_code=$(echo "${response}" | tail -1 | sed 's/.*HTTP_CODE://')
    local response_body
    response_body=$(echo "${response}" | sed '$d')

    if [[ "${http_code}" -eq 201 ]]; then
        log "Release created successfully!"
        log "URL: ${CI_PROJECT_URL}/-/releases/${CI_COMMIT_TAG}"
    else
        error "Failed to create release (HTTP ${http_code})\n${response_body}"
    fi
}

#----------------------------------------------------------
# Main
#----------------------------------------------------------

main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --preview)
                PREVIEW_MODE=true
                shift
                # Check if next arg is a version (not another flag)
                if [[ $# -gt 0 && "$1" != -* ]]; then
                    setup_preview_environment "$1"
                    shift
                else
                    setup_preview_environment ""
                fi
                ;;
            --help|-h)
                usage
                ;;
            *)
                error "Unknown argument: $1"
                ;;
        esac
    done

    log "Starting release process"
    log "Tag: ${CI_COMMIT_TAG:-<not set>}"

    cd "${REPO_ROOT}"

    # Validate environment
    validate_environment

    # Extract version from tag (strip 'v' prefix)
    local version="${CI_COMMIT_TAG#v}"
    log "Version: ${version}"

    # Extract changelog section for this version from CHANGELOG.md
    # Falls back to git log if version not found
    extract_changelog_section "${version}"

    # Generate release notes (includes changelog section)
    generate_release_notes "${version}"

    if [[ "${PREVIEW_MODE}" == "true" ]]; then
        log "Preview mode - skipping GitLab API calls"
        echo ""
        echo "================================================================================"
        echo "RELEASE NOTES PREVIEW"
        echo "================================================================================"
        cat "${REPO_ROOT}/RELEASE_NOTES.md"
        echo "================================================================================"
        log "Preview complete. Files generated:"
        log "  - ${REPO_ROOT}/RELEASE_NOTES.md"
        log "  - ${REPO_ROOT}/RELEASE_CHANGELOG.md"
    else
        # Upload artifacts to package registry (public access)
        upload_to_package_registry "${version}"

        # Create GitLab release with links to package registry
        create_gitlab_release "${version}"
        log "Release process complete"
    fi
}

main "$@"

#!/usr/bin/env bash
#----------------------------------------------------------
# Shebe CI Release Script
#
# Creates a GitLab release with changelog and artifact links.
# Uses CI_JOB_TOKEN for authentication (no manual token needed).
#
# Usage:
#   ./scripts/ci-release.sh
#
# Required environment variables (GitLab CI predefined):
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Configuration
RELEASE_DIR="${RELEASE_DIR:-releases}"

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

validate_environment() {
    log "Validating environment..."

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

# Extract changelog section for a specific version from CHANGELOG.md
# Falls back to git log if version not found in CHANGELOG.md
extract_changelog_section() {
    local version="$1"
    local changelog_file="${REPO_ROOT}/CHANGELOG.md"
    local output_file="${REPO_ROOT}/RELEASE_CHANGELOG.md"

    log "Extracting changelog for version ${version}..."

    if [[ -f "${changelog_file}" ]]; then
        # Extract section between [version] and the next ## [ or end of file
        # Using awk to extract the section for this version
        local section
        section=$(awk -v ver="${version}" '
            /^## \[/ {
                if (found) exit
                if (index($0, "[" ver "]") > 0) found=1
            }
            found { print }
        ' "${changelog_file}")

        if [[ -n "${section}" ]]; then
            echo "${section}" > "${output_file}"
            log "Extracted changelog section from CHANGELOG.md"
            return 0
        fi
    fi

    # Fallback: generate from git log
    log "Version not found in CHANGELOG.md, generating from git history..."
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

    cat > "${release_notes_file}" << EOF
# Shebe ${CI_COMMIT_TAG}

**Release Date:** $(date -u +"%Y-%m-%d")
**Commit:** [\`${CI_COMMIT_SHORT_SHA}\`](${CI_PROJECT_URL}/-/commit/${CI_COMMIT_SHA})

## Downloads

| Platform | Download | Checksum |
|----------|----------|----------|
| Linux x86_64 | [shebe-v${version}-linux-x86_64.tar.gz](${CI_PROJECT_URL}/-/jobs/artifacts/${CI_COMMIT_TAG}/raw/releases/shebe-v${version}-linux-x86_64.tar.gz?job=build:shebe) | [SHA256](${CI_PROJECT_URL}/-/jobs/artifacts/${CI_COMMIT_TAG}/raw/releases/shebe-v${version}-linux-x86_64.tar.gz.sha256?job=build:shebe) |

## Installation

\`\`\`bash
# Download and extract
curl -LO "${CI_PROJECT_URL}/-/jobs/artifacts/${CI_COMMIT_TAG}/raw/releases/shebe-v${version}-linux-x86_64.tar.gz?job=build:shebe"
tar -xzf shebe-v${version}-linux-x86_64.tar.gz

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

    # Build release payload
    local payload
    payload=$(jq -n \
        --arg tag "${CI_COMMIT_TAG}" \
        --arg name "Shebe ${CI_COMMIT_TAG}" \
        --arg description "$(cat "${release_notes_file}")" \
        --arg ref "${CI_COMMIT_SHA}" \
        --arg tarball_url "${CI_PROJECT_URL}/-/jobs/artifacts/${CI_COMMIT_TAG}/raw/releases/shebe-v${version}-linux-x86_64.tar.gz?job=build:shebe" \
        --arg checksum_url "${CI_PROJECT_URL}/-/jobs/artifacts/${CI_COMMIT_TAG}/raw/releases/shebe-v${version}-linux-x86_64.tar.gz.sha256?job=build:shebe" \
        '{
            tag_name: $tag,
            name: $name,
            description: $description,
            ref: $ref,
            assets: {
                links: [
                    {
                        name: "shebe-linux-x86_64.tar.gz",
                        url: $tarball_url,
                        link_type: "package"
                    },
                    {
                        name: "shebe-linux-x86_64.tar.gz.sha256",
                        url: $checksum_url,
                        link_type: "other"
                    }
                ]
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

    # Create GitLab release
    create_gitlab_release "${version}"

    log "Release process complete"
}

main "$@"

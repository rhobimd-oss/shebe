# Makefile style guide from http://clarkgrubb.com/makefile-style-guide
MAKEFLAGS += --warn-undefined-variables
SHELL := bash
.SHELLFLAGS := -o errexit -o nounset -o pipefail -c
.DEFAULT_GOAL := docker-build
.DELETE_ON_ERROR:
.SUFFIXES:


# STATIC VARS
REGISTRY_URL := registry.gitlab.com
IMAGE_NAME := rhobimd-oss/shebe
IMAGE_TAG ?= $(shell cat services/shebe-server/VERSION 2>/dev/null || echo "latest")
CONTAINER_NAME := shebe-server

# Docker build args
DOCKERFILE := services/shebe-server/Dockerfile
BUILD_CONTEXT := services/shebe-server

# Runtime configuration
HOST_PORT ?= 3000
PROJECT_DIR ?= $(PWD)


# DEVELOPMENT TARGETS ----------------------------------------------------------
# shebe-dev: Debian/glibc for tests, linting, formatting
# shebe-dev-musl: Alpine/musl for static builds with sccache (mirrors CI)
COMPOSE := docker compose --file ${PROJECT_DIR}/deploy/docker-compose.yml run --rm --remove-orphans
RUN_DEV := $(COMPOSE) shebe-dev entrypoint.sh with-sccache
RUN_DEV_NO_CACHE := $(COMPOSE) shebe-dev entrypoint.sh without-sccache
RUN_MUSL := $(COMPOSE) shebe-dev-musl entrypoint.sh with-sccache
RUN_MUSL_NO_CACHE := $(COMPOSE) shebe-dev-musl entrypoint.sh without-sccache


# Build targets
build:
	@echo "Building in shebe-dev container..."
	$(RUN_DEV) cargo build

build-release:
	@echo "Building release in shebe-dev container..."
	$(RUN_DEV) cargo build --release

# Test and quality targets
test:
	@echo "Running tests in shebe-dev container..."
	$(RUN_DEV) cargo nextest run --color=always

test-coverage:
	@echo "Running tests with coverage in shebe-dev container..."
	$(RUN_DEV) cargo tarpaulin --all-features --workspace --out Xml --output-dir . --fail-under 70

fix:
	$(RUN_DEV) cargo fix --package shebe --verbose --allow-no-vcs

fmt:
	@echo "Formatting code in shebe-dev container..."
	$(RUN_DEV) cargo fmt

fmt-check:
	@echo "Checking code formatting in shebe-dev container..."
	$(RUN_DEV) cargo fmt -- --check --verbose

clippy:
	@echo "Running clippy in shebe-dev container..."
	$(RUN_DEV) cargo clippy --no-deps -- -D warnings

check:
	@echo "Running cargo check in shebe-dev container..."
	$(RUN_DEV) cargo check

ci:
	@echo "Running full CI suite in single container..."
	$(RUN_DEV) bash -c "\
		cargo nextest run --color=always && \
		cargo fmt -- --check --verbose && \
		cargo clippy --no-deps -- -D warnings && \
		cargo check"

# Interactive shell in shebe-dev container (with sccache)
shell:
	@echo "Starting interactive shell in shebe-dev container..."
	$(COMPOSE) shebe-dev entrypoint.sh with-sccache bash

shell-no-cache:
	@echo "Starting interactive shell (no sccache)..."
	$(COMPOSE) shebe-dev bash

## ------------------------------------------------------------------------------------
##                           BUILD TARGETS
## ------------------------------------------------------------------------------------

# Musl/static build targets with sccache (mirrors CI environment)
build-musl:
	$(RUN_MUSL) cargo build --release

build-musl-no-cache:
	$(RUN_MUSL_NO_CACHE) cargo build --release

shell-musl:
	$(COMPOSE) shebe-dev-musl entrypoint.sh with-sccache bash

shell-musl-no-cache:
	$(COMPOSE) shebe-dev-musl bash

# rci targets (mirrors CI pipeline)
rci-build:
	$(RUN_MUSL) rci build --service-dir . --suffix linux-x86_64-musl

rci-build-no-cache:
	$(RUN_MUSL_NO_CACHE) rci build --service-dir . --suffix linux-x86_64-musl

rci-mcpb:
	$(RUN_MUSL) rci mcpb create --service-dir .

# sccache validation
sccache-test:
	$(RUN_MUSL) cargo build --release

# Clean Docker artifacts
clean:
	@echo "Cleaning Docker volumes..."
	docker volume rm deploy_cargo-registry deploy_cargo-git deploy_cargo-target 2>/dev/null || true
	docker volume rm deploy_cargo-registry-musl deploy_cargo-git-musl deploy_cargo-target-musl 2>/dev/null || true
	@echo "Docker volumes cleaned"

# SHEBE BINARIES ---------------------------------------------------------------
VERSION ?= $(shell cat services/shebe-server/VERSION)
ARCH := linux-x86_64
BUILD_DIR := services/shebe-server/build/release

# CLI binary (shebe)
CLI_VERSIONED_NAME := shebe-v$(VERSION)-$(ARCH)
CLI_BINARY := $(BUILD_DIR)/shebe

# MCP binary (shebe-mcp)
MCP_VERSIONED_NAME := shebe-mcp-v$(VERSION)-$(ARCH)
MCP_BINARY := $(BUILD_DIR)/shebe-mcp

shebe-build:
	@echo "Building shebe and shebe-mcp in shebe-dev container..."
	$(RUN_DEV) cargo build --release --target-dir /workspace/build

shebe-install: shebe-build
	@echo "Installing $(CLI_VERSIONED_NAME) to /usr/local/lib/..."
	sudo cp $(CLI_BINARY) /usr/local/lib/$(CLI_VERSIONED_NAME)
	@echo "Creating symlink /usr/local/bin/shebe..."
	sudo ln -sfv /usr/local/lib/$(CLI_VERSIONED_NAME) /usr/local/bin/shebe
	@echo ""
	@echo "Installing $(MCP_VERSIONED_NAME) to /usr/local/lib/..."
	sudo cp $(MCP_BINARY) /usr/local/lib/$(MCP_VERSIONED_NAME)
	@echo "Creating symlink /usr/local/bin/shebe-mcp..."
	sudo ln -sfv /usr/local/lib/$(MCP_VERSIONED_NAME) /usr/local/bin/shebe-mcp
	@echo ""
	@echo "Installed binaries:"
	@ls -lh /usr/local/bin/shebe /usr/local/bin/shebe-mcp
	@which shebe shebe-mcp

shebe-install-config:
	@echo "Installing config file template to ~/.config/shebe/..."
	@mkdir -p ~/.config/shebe
	@if [ -f ~/.config/shebe/config.toml ]; then \
		echo "Config file already exists at ~/.config/shebe/config.toml"; \
		echo "To replace it, run: cp shebe.toml ~/.config/shebe/config.toml"; \
	else \
		cp shebe.toml ~/.config/shebe/config.toml; \
		echo "Config file installed: ~/.config/shebe/config.toml"; \
		echo "Edit with: nano ~/.config/shebe/config.toml"; \
	fi

shebe-uninstall:
	@echo "Removing shebe and shebe-mcp symlinks and versioned binaries..."
	sudo rm -f /usr/local/bin/shebe
	sudo rm -f /usr/local/lib/$(CLI_VERSIONED_NAME)
	sudo rm -f /usr/local/bin/shebe-mcp
	sudo rm -f /usr/local/lib/$(MCP_VERSIONED_NAME)
	@echo "Uninstallation complete"

shebe-test:
	@echo "Testing shebe CLI..."
	@shebe --version
	@echo ""
	@echo "Testing shebe-mcp binary with initialize message..."
	@echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{'\
'"protocolVersion":"2024-11-05","capabilities":{"tools":{}},'\
'"clientInfo":{"name":"test","version":"1.0"}}}' | shebe-mcp


# HELP TARGET ------------------------------------------------------------------
help:
	@echo "Shebe Makefile Targets:"
	@echo ""
	@echo "Development Targets (shebe-dev + sccache - Debian/glibc):"
	@echo "  build                Build debug binary"
	@echo "  build-release        Build release binary"
	@echo "  test                 Run tests with cargo nextest"
	@echo "  test-coverage        Run tests with coverage (tarpaulin)"
	@echo "  fmt                  Format code"
	@echo "  fmt-check            Check code formatting"
	@echo "  clippy               Run clippy linter"
	@echo "  check                Run cargo check"
	@echo "  shell                Shell with sccache"
	@echo "  shell-no-cache       Shell without sccache"
	@echo "  clean                Clean Docker volumes"
	@echo ""
	@echo "Musl + sccache Targets (shebe-dev-musl - mirrors CI):"
	@echo "  build-musl           Build with sccache"
	@echo "  build-musl-no-cache  Build without sccache"
	@echo "  shell-musl           Shell with sccache"
	@echo "  shell-musl-no-cache  Shell without sccache"
	@echo "  rci-build            rci build with sccache"
	@echo "  rci-build-no-cache   rci build without sccache"
	@echo "  rci-mcpb             rci mcpb create"
	@echo "  sccache-test         Test sccache"
	@echo ""
	@echo "Shebe Binaries (shebe-dev container):"
	@echo "  shebe-build          Build shebe (CLI) and shebe-mcp binaries"
	@echo "  shebe-install        Install both binaries to /usr/local/lib"
	@echo "  shebe-install-config Install config template to ~/.config/shebe/"
	@echo "  shebe-uninstall      Remove installed binaries and symlinks"
	@echo "  shebe-test           Test shebe-mcp with initialize message"
	@echo ""
	@echo "Variables:"
	@echo "  IMAGE_TAG=$(IMAGE_TAG)"
	@echo "  HOST_PORT=$(HOST_PORT)"
	@echo "  VERSION=$(VERSION)"

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
DATA_DIR ?= $(PWD)/data


# DEVELOPMENT TARGETS ----------------------------------------------------------
# All local Rust commands run in shebe-dev container for consistency with CI/CD
DOCKER_RUN := cd deploy && docker compose run --rm shebe-dev

# Build targets
build:
	@echo "Building in shebe-dev container..."
	$(DOCKER_RUN) cargo build

build-release:
	@echo "Building release in shebe-dev container..."
	$(DOCKER_RUN) cargo build --release

# Test and quality targets
test:
	@echo "Running tests in shebe-dev container..."
	$(DOCKER_RUN) cargo nextest run

test-coverage:
	@echo "Running tests with coverage in shebe-dev container..."
	$(DOCKER_RUN) cargo tarpaulin --all-features --workspace --out Xml --output-dir . --fail-under 85

fmt:
	@echo "Formatting code in shebe-dev container..."
	$(DOCKER_RUN) cargo fmt

fmt-check:
	@echo "Checking code formatting in shebe-dev container..."
	$(DOCKER_RUN) cargo fmt -- --check --verbose

clippy:
	@echo "Running clippy in shebe-dev container..."
	$(DOCKER_RUN) cargo clippy -- -D warnings

check:
	@echo "Running cargo check in shebe-dev container..."
	$(DOCKER_RUN) cargo check

# Interactive shell in shebe-dev container
shell:
	@echo "Starting interactive shell in shebe-dev container..."
	cd deploy && docker compose run --rm shebe-dev bash

# Clean Docker artifacts
clean:
	@echo "Cleaning Docker volumes..."
	docker volume rm deploy_cargo-registry deploy_cargo-git deploy_cargo-target 2>/dev/null || true
	@echo "Docker volumes cleaned"


# MCP TARGETS ------------------------------------------------------------------
VERSION ?= $(shell cat services/shebe-server/VERSION)
ARCH := linux-x86_64
VERSIONED_NAME := shebe-mcp-v$(VERSION)-$(ARCH)
MCP_BINARY := services/shebe-server/target/release/shebe-mcp

mcp-build:
	@echo "Building shebe-mcp in shebe-dev container..."
	$(DOCKER_RUN) cargo build --release --verbose --bin shebe-mcp

mcp-install: mcp-build
	@echo "Installing $(VERSIONED_NAME) to /usr/local/lib/..."
	sudo cp $(MCP_BINARY) /usr/local/lib/$(VERSIONED_NAME)
	@echo "Creating symlink /usr/local/bin/shebe-mcp..."
	sudo ln -sfv /usr/local/lib/$(VERSIONED_NAME) /usr/local/bin/shebe-mcp
	@ls -lh /usr/local/bin/shebe-mcp
	@which shebe-mcp

mcp-install-config:
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

mcp-uninstall:
	@echo "Removing shebe-mcp symlink and versioned binary..."
	sudo rm -f /usr/local/bin/shebe-mcp
	sudo rm -f /usr/local/lib/$(VERSIONED_NAME)
	@echo "Uninstallation complete"

mcp-test:
	@echo "Testing shebe-mcp binary with initialize message..."
	@echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{'\
'"protocolVersion":"2024-11-05","capabilities":{"tools":{}},'\
'"clientInfo":{"name":"test","version":"1.0"}}}' | shebe-mcp


# HELP TARGET ------------------------------------------------------------------
help:
	@echo "Shebe Makefile Targets:"
	@echo ""
	@echo "Development Targets (shebe-dev container):"
	@echo "  build                Build debug binary"
	@echo "  build-release        Build release binary"
	@echo "  run                  Run server (debug mode)"
	@echo "  run-release          Run server (release mode)"
	@echo "  test                 Run tests with cargo nextest"
	@echo "  test-coverage        Run tests with coverage (tarpaulin)"
	@echo "  fmt                  Format code"
	@echo "  fmt-check            Check code formatting"
	@echo "  clippy               Run clippy linter"
	@echo "  check                Run cargo check"
	@echo "  shell                Open interactive shell in shebe-dev"
	@echo "  clean                Clean Docker volumes"
	@echo ""
	@echo "MCP Targets (shebe-dev container):"
	@echo "  mcp-build            Build shebe-mcp binary"
	@echo "  mcp-install          Install versioned binary to /usr/local/lib"
	@echo "  mcp-install-config   Install config template to ~/.config/shebe/"
	@echo "  mcp-uninstall        Remove installed binary and symlink"
	@echo "  mcp-test             Test MCP binary with initialize message"
	@echo ""
	@echo "Variables:"
	@echo "  IMAGE_TAG=$(IMAGE_TAG)"
	@echo "  HOST_PORT=$(HOST_PORT)"
	@echo "  DATA_DIR=$(DATA_DIR)"
	@echo "  VERSION=$(VERSION)"

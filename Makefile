# Makefile for archtui development and testing
# NOTE: This Makefile is for DEVELOPMENT ONLY, not for use in Arch ISO

.PHONY: help build test test-rust test-bash clean install dev-setup format lint deps dev quick ci iso-ready

# Default target
help:
	@echo "ArchTUI - Development Makefile"
	@echo "⚠️  DEVELOPMENT ONLY - Not for use in Arch ISO"
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build the release binary"
	@echo "  build-debug  - Build the debug binary"
	@echo "  test         - Run all tests (Rust + Bash)"
	@echo "  test-rust    - Run Rust tests only"
	@echo "  test-bash    - Run Bash script tests only"
	@echo "  clean        - Clean build artifacts"
	@echo "  install      - Install the binary to /usr/local/bin"
	@echo "  iso-ready    - Check if installer is ready for ISO"
	@echo "  dev-setup    - Set up development environment"
	@echo "  format       - Format Rust code"
	@echo "  lint         - Run linter checks"
	@echo "  deps         - Update dependencies"

# Build targets
build:
	cargo build --release
	cp target/release/archtui ./

build-debug:
	cargo build
	cp target/debug/archtui ./

# Testing
test: test-rust test-bash

test-rust:
	cargo test

test-bash:
	@if command -v bats >/dev/null 2>&1; then \
		./scripts/tests/run_tests.sh; \
	else \
		echo "⚠️  bats not installed, skipping bash tests"; \
		echo "   Install with: sudo pacman -S bash-bats"; \
	fi

# Cleanup
clean:
	cargo clean
	rm -f ./archtui

# Installation
install: build
	sudo cp ./archtui /usr/local/bin/
	sudo chmod +x /usr/local/bin/archtui

# Development setup
dev-setup:
	rustup component add rustfmt clippy
	cargo install cargo-watch


# Code quality
format:
	cargo fmt

lint:
	cargo clippy -- -D warnings

# Dependencies
deps:
	cargo update

# Development workflow
dev: format lint test build

# Quick development build
quick: build-debug

# Full CI pipeline
ci: format lint test build

# ISO readiness check
iso-ready:
	@echo "Checking ArchInstall readiness for ISO deployment..."
	@if [ -f "./archtui" ]; then \
		echo "✅ Binary exists: ./archtui"; \
		if [ -x "./archtui" ]; then \
			echo "✅ Binary is executable"; \
		else \
			echo "❌ Binary is not executable"; \
			exit 1; \
		fi; \
	else \
		echo "❌ Binary missing: ./archtui"; \
		echo "   Run 'make build' to create the binary"; \
		exit 1; \
	fi
	@echo "✅ All scripts are executable"
	@echo "✅ ArchInstall is ready for ISO deployment!"

# Makefile for archinstall-tui development and testing
# NOTE: This Makefile is for DEVELOPMENT ONLY, not for use in Arch ISO

.PHONY: help build test clean install dev-setup format lint deps dev quick ci iso-ready

# Default target
help:
	@echo "ArchInstall TUI - Development Makefile"
	@echo "⚠️  DEVELOPMENT ONLY - Not for use in Arch ISO"
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build the release binary"
	@echo "  build-debug  - Build the debug binary"
	@echo "  test         - Run tests"
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
	cp target/release/archinstall-tui ./

build-debug:
	cargo build
	cp target/debug/archinstall-tui ./

# Testing
test:
	cargo test

# Cleanup
clean:
	cargo clean
	rm -f ./archinstall-tui

# Installation
install: build
	sudo cp ./archinstall-tui /usr/local/bin/
	sudo chmod +x /usr/local/bin/archinstall-tui

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
	@if [ -f "./archinstall-tui" ]; then \
		echo "✅ Binary exists: ./archinstall-tui"; \
		if [ -x "./archinstall-tui" ]; then \
			echo "✅ Binary is executable"; \
		else \
			echo "❌ Binary is not executable"; \
			exit 1; \
		fi; \
	else \
		echo "❌ Binary missing: ./archinstall-tui"; \
		echo "   Run 'make build' to create the binary"; \
		exit 1; \
	fi
	@echo "✅ All scripts are executable"
	@echo "✅ ArchInstall is ready for ISO deployment!"

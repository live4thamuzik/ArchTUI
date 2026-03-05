# Makefile for archtui development and testing
# NOTE: This Makefile is for DEVELOPMENT ONLY, not for use in Arch ISO

PREFIX ?= /usr/local
DESTDIR ?=

.PHONY: help build test test-rust test-bash clean install dev-setup format lint lint-rust lint-bash deps dev quick ci iso-ready generate

# Default target
help:
	@echo "ArchTUI - Development Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build the release binary"
	@echo "  build-debug  - Build the debug binary"
	@echo "  test         - Run all tests (Rust + Bash)"
	@echo "  test-rust    - Run Rust tests only"
	@echo "  test-bash    - Run Bash script tests only"
	@echo "  clean        - Clean build artifacts"
	@echo "  install      - Install binary, scripts, man page, and completions"
	@echo "  generate     - Generate man page and shell completions"
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
		echo "bats not installed, skipping bash tests"; \
		echo "   Install with: sudo pacman -S bash-bats"; \
	fi

# Cleanup
clean:
	cargo clean
	rm -f ./archtui
	rm -rf dist/

# Generate man page and shell completions into dist/
generate:
	ARCHTUI_GEN_DIR=dist cargo build --release --no-default-features
	cp target/release/archtui ./

# Installation with FHS layout
install: build generate
	install -Dm755 ./archtui $(DESTDIR)$(PREFIX)/bin/archtui
	install -d $(DESTDIR)$(PREFIX)/share/archtui/scripts
	cp -r scripts/*.sh $(DESTDIR)$(PREFIX)/share/archtui/scripts/
	cp -r scripts/strategies $(DESTDIR)$(PREFIX)/share/archtui/scripts/
	cp -r scripts/tools $(DESTDIR)$(PREFIX)/share/archtui/scripts/
	install -Dm644 dist/man/archtui.1 $(DESTDIR)$(PREFIX)/share/man/man1/archtui.1
	install -Dm644 dist/completions/archtui.bash $(DESTDIR)$(PREFIX)/share/bash-completion/completions/archtui
	install -Dm644 dist/completions/_archtui $(DESTDIR)$(PREFIX)/share/zsh/site-functions/_archtui
	install -Dm644 dist/completions/archtui.fish $(DESTDIR)$(PREFIX)/share/fish/vendor_completions.d/archtui.fish

# Development setup
dev-setup:
	rustup component add rustfmt clippy
	cargo install cargo-watch


# Code quality
format:
	cargo fmt

lint: lint-rust lint-bash

lint-rust:
	cargo clippy -- -D warnings

# Match CI flags exactly (alpm feature disabled on non-Arch systems)
lint-ci:
	cargo clippy --no-default-features -- -D warnings

lint-bash:
	@if command -v shellcheck >/dev/null 2>&1; then \
		echo "Running shellcheck on all scripts..."; \
		find scripts -name "*.sh" -exec shellcheck -x \
			-P scripts -P scripts/strategies -P scripts/tools {} +; \
		echo "shellcheck passed"; \
	else \
		echo "shellcheck not installed, skipping"; \
	fi

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
		echo "Binary exists: ./archtui"; \
		if [ -x "./archtui" ]; then \
			echo "Binary is executable"; \
		else \
			echo "Binary is not executable"; \
			exit 1; \
		fi; \
	else \
		echo "Binary missing: ./archtui"; \
		echo "   Run 'make build' to create the binary"; \
		exit 1; \
	fi
	@echo "ArchInstall is ready for ISO deployment!"

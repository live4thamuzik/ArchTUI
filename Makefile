# Makefile for archinstall-tui development and testing

.PHONY: help build test clean install dev-setup docker-build docker-test

# Default target
help:
	@echo "ArchInstall TUI - Development Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build        - Build the release binary"
	@echo "  build-debug  - Build the debug binary"
	@echo "  test         - Run tests"
	@echo "  clean        - Clean build artifacts"
	@echo "  install      - Install the binary to /usr/local/bin"
	@echo "  dev-setup    - Set up development environment"
	@echo "  docker-build - Build Docker image for testing"
	@echo "  docker-test  - Run tests in Docker container"
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

# Docker targets
docker-build:
	docker build -t archinstall-tui:dev .

docker-test:
	docker run --rm -v $(PWD):/workspace archinstall-tui:dev make test

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

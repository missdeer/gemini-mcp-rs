.PHONY: help build build-release test fmt clippy clean install uninstall check-version

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build debug binary
	cargo build

build-release: ## Build release binary
	cargo build --release

test: ## Run all tests
	cargo test --all-features

test-unit: ## Run unit tests only
	cargo test --lib

test-integration: ## Run integration tests only
	cargo test --test '*'

test-doc: ## Run documentation tests
	cargo test --doc

test-coverage: ## Generate test coverage report
	cargo tarpaulin --out Html --out Xml --all-features

test-watch: ## Run tests in watch mode (requires cargo-watch)
	cargo watch -x test

fmt: ## Format code
	cargo fmt

clippy: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

clean: ## Clean build artifacts
	cargo clean
	rm -rf npm/gemini-mcp-rs/node_modules
	rm -f npm/gemini-mcp-rs/*.tgz npm/gemini-mcp-rs/*.tar.gz npm/gemini-mcp-rs/*.zip
	rm -rf npm/platforms/*/bin npm/platforms/*/node_modules
	rm -f npm/platforms/*/*.tgz

PREFIX ?= /usr/local
DESTDIR ?=

install: build-release ## Install binary to $(DESTDIR)$(PREFIX)/bin
	@mkdir -p $(DESTDIR)$(PREFIX)/bin
	@cp target/release/gemini-mcp-rs $(DESTDIR)$(PREFIX)/bin/gemini-mcp-rs
	@chmod 755 $(DESTDIR)$(PREFIX)/bin/gemini-mcp-rs
	@echo "Installed gemini-mcp-rs to $(DESTDIR)$(PREFIX)/bin"

uninstall: ## Uninstall binary from $(DESTDIR)$(PREFIX)/bin
	@rm -f $(DESTDIR)$(PREFIX)/bin/gemini-mcp-rs
	@echo "Uninstalled gemini-mcp-rs from $(DESTDIR)$(PREFIX)/bin"

check-version: ## Check version consistency across files
	@bash scripts/check-version.sh

check: fmt clippy test ## Run all checks (fmt, clippy, test)

ci: check build-release ## Run all CI checks

npm-pack: build-release ## Pack npm package for testing
	cd npm/gemini-mcp-rs && npm pack

npm-install: npm-pack ## Install npm package locally for testing
	npm install -g npm/gemini-mcp-rs/missdeer-gemini-mcp-rs-*.tgz


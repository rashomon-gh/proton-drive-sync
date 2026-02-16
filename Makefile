.PHONY: help build install uninstall service-install service-uninstall clean test fmt clippy

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the project in release mode
	cargo build --release

install: build ## Install the binary to /usr/local/bin
	sudo cp target/release/proton-drive-sync /usr/local/bin/
	@echo "Binary installed to /usr/local/bin/proton-drive-sync"

uninstall: ## Remove the binary from /usr/local/bin
	sudo rm -f /usr/local/bin/proton-drive-sync
	@echo "Binary removed from /usr/local/bin/proton-drive-sync"

service-install: ## Install systemd user service
	cd packaging/systemd && ./install-service.sh

service-uninstall: ## Uninstall systemd user service
	systemctl --user stop proton-drive-sync || true
	systemctl --user disable proton-drive-sync || true
	rm -f ~/.config/systemd/user/proton-drive-sync@.service
	systemctl --user daemon-reload
	@echo "Systemd service uninstalled"

clean: ## Remove build artifacts
	cargo clean

test: ## Run all tests
	cargo test

test-coverage: ## Run tests with coverage
	cargo llvm-cov --all-features --lcov --output-path lcov.info

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

clippy: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

check: fmt-check clippy test ## Run all checks (format, clippy, tests)

.DEFAULT_GOAL := help

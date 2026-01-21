# BitQuan Blockchain - Makefile
# Common development tasks for BitQuan cryptocurrency implementation

.PHONY: all build check test clippy fmt clean run benchmark help

# Default target
all: build

## Build targets
build: ## Build all crates in debug mode
	cargo build --all-features

release: ## Build all crates in release mode
	cargo build --release --all-features

## Code quality targets
check: ## Run quick checks (format, clippy)
	@echo "Running cargo fmt --check..."
	cargo fmt --all -- --check
	@echo "Running cargo clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

clippy: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

fmt: ## Format code with rustfmt
	cargo fmt --all

fmt-check: ## Check if code is formatted
	cargo fmt --all -- --check

## Testing targets
test: ## Run all tests
	cargo test --all-features

test-quiet: ## Run tests with minimal output
	cargo test --all-features --quiet

test-fast: ## Run tests without all features (faster)
	cargo test

## Security targets
audit: ## Run cargo deny check
	cargo deny check

audit-advisories: ## Check security advisories
	cargo deny check advisories

audit-bans: ## Check dependency bans
	cargo deny check bans

audit-licenses: ## Check license compliance
	cargo deny check licenses

audit-sources: ## Check source validity
	cargo deny check sources

## Documentation targets
doc: ## Generate and open documentation
	cargo doc --all-features --open

doc-no-deps: ## Generate documentation without dependencies
	cargo doc --all-features --no-deps

## Cleaning targets
clean: ## Clean build artifacts
	cargo clean

clean-all: clean ## Clean everything including temporary files
	rm -rf data/* .agents

## Development targets
run: ## Run the main node
	cargo run --bin bitquan-node --all-features

run-miner: ## Run the miner
	cargo run --bin bitquan-node --all-features -- mine

run-faucet: ## Run the faucet server
	cargo run --bin faucet --all-features

## Utility targets
update: ## Update dependencies
	cargo update

update-deps: update ## Update all dependencies (alias)

tree: ## Show dependency tree
	cargo tree

fetch: ## Fetch all git dependencies
	cargo fetch

## CI targets (used by GitHub Actions)
ci: ## Run full CI checks (format, clippy, test, audit)
	@echo "=== Running cargo fmt --check ==="
	cargo fmt --all -- --check
	@echo "=== Running cargo clippy ==="
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "=== Running cargo test ==="
	cargo test --all-features
	@echo "=== Running cargo deny check ==="
	cargo deny check
	@echo "=== All CI checks passed! ==="

ck: ci ## Alias for ci (pre-commit check)

## Help target
help: ## Show this help message
	@echo "BitQuan Development Commands"
	@echo ""
	@echo "Usage:"
	@echo "  make <target>"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}'
	@echo ""
	@echo "Examples:"
	@echo "  make build           # Build all crates"
	@echo "  make test            # Run all tests"
	@echo "  make clippy          # Run linter"
	@echo "  make fmt             # Format code"
	@echo "  make ci              # Run full CI checks"
	@echo "  make help            # Show this help"

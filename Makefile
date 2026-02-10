.PHONY: help build build-release test test-all clean fmt fmt-check lint doc doc-open examples check check-msrv install-tools ci-local download-test-data

# Default target - show help
help:
	@echo "OpenVDS-Rust Makefile Commands"
	@echo "==============================="
	@echo ""
	@echo "Development:"
	@echo "  make build           - Build the project in debug mode"
	@echo "  make build-release   - Build the project in release mode"
	@echo "  make test            - Run tests (no default features)"
	@echo "  make test-all        - Run all tests with all feature combinations"
	@echo "  make examples        - Run all examples"
	@echo "  make check           - Quick compile check"
	@echo ""
	@echo "Code Quality:"
	@echo "  make fmt             - Format code with rustfmt"
	@echo "  make fmt-check       - Check code formatting"
	@echo "  make lint            - Run clippy linter"
	@echo "  make lint-fix        - Auto-fix clippy warnings"
	@echo ""
	@echo "Documentation:"
	@echo "  make doc             - Build documentation"
	@echo "  make doc-open        - Build and open documentation"
	@echo ""
	@echo "Test Data:"
	@echo "  make download-test-data - Download sample VDS files from OpenVDS repository"
	@echo ""
	@echo "Verification:"
	@echo "  make check-msrv      - Verify MSRV (Rust 1.70) compatibility"
	@echo "  make ci-local        - Run all CI checks locally"
	@echo ""
	@echo "Utilities:"
	@echo "  make clean           - Clean build artifacts"
	@echo "  make install-tools   - Install required development tools"

# Build commands
build:
	@echo "Building openvds-rs (debug)..."
	cargo build --no-default-features

build-release:
	@echo "Building openvds-rs (release)..."
	cargo build --release --no-default-features

# Test commands
test:
	@echo "Running tests..."
	cargo test --no-default-features

test-all:
	@echo "Running tests with no features..."
	cargo test --no-default-features
	@echo ""
	@echo "Running tests with default features..."
	cargo test
	@echo ""
	@echo "Running tests with all features..."
	cargo test --all-features
	@echo ""
	@echo "Running doc tests..."
	cargo test --doc

# Check commands
check:
	@echo "Running cargo check..."
	cargo check --no-default-features

check-msrv:
	@echo "Checking MSRV compatibility (Rust 1.70)..."
	@if command -v rustup >/dev/null 2>&1; then \
		rustup toolchain install 1.70; \
		cargo +1.70 check --no-default-features; \
		cargo +1.70 test --no-default-features; \
	else \
		echo "Error: rustup not found. Please install rustup to verify MSRV."; \
		exit 1; \
	fi

# Format commands
fmt:
	@echo "Formatting code..."
	cargo fmt --all

fmt-check:
	@echo "Checking code formatting..."
	cargo fmt --all -- --check

# Lint commands
lint:
	@echo "Running clippy (no features)..."
	cargo clippy --no-default-features --all-targets -- -D warnings
	@echo ""
	@echo "Running clippy (default features)..."
	cargo clippy --all-targets -- -D warnings
	@echo ""
	@echo "Running clippy (all features)..."
	cargo clippy --all-features --all-targets -- -D warnings

lint-fix:
	@echo "Auto-fixing clippy warnings..."
	cargo clippy --fix --no-default-features --allow-dirty --allow-staged

# Documentation commands
doc:
	@echo "Building documentation..."
	cargo doc --no-deps --all-features

doc-open:
	@echo "Building and opening documentation..."
	cargo doc --no-deps --all-features --open

# Examples
examples:
	@echo "Running seismic_volume example..."
	cargo run --example seismic_volume
	@echo ""
	@echo "Running concurrent_loading example..."
	cargo run --example concurrent_loading

# Download test data
download-test-data:
	@echo "Downloading sample VDS files from OpenVDS repository..."
	@mkdir -p test-data
	@echo "Downloading from: https://community.opengroup.org/osdu/platform/domain-data-mgmt-services/seismic/open-vds/-/tree/master/tests/VDS"
	@echo ""
	@if command -v git >/dev/null 2>&1; then \
		if [ ! -d "test-data/open-vds" ]; then \
			echo "Cloning OpenVDS repository (sparse checkout for test files only)..."; \
			git clone --depth 1 --filter=blob:none --sparse \
				https://community.opengroup.org/osdu/platform/domain-data-mgmt-services/seismic/open-vds.git \
				test-data/open-vds; \
			cd test-data/open-vds && git sparse-checkout set tests/VDS; \
			cd ../..; \
			echo ""; \
			echo "✓ Test data downloaded to: test-data/open-vds/tests/VDS/"; \
			echo ""; \
			echo "Available test files:"; \
			find test-data/open-vds/tests/VDS -type f -name "*.vds" 2>/dev/null || echo "  (scanning...)"; \
		else \
			echo "Test data already exists at: test-data/open-vds/tests/VDS/"; \
			echo "To re-download, run: rm -rf test-data/open-vds && make download-test-data"; \
		fi \
	elif command -v curl >/dev/null 2>&1; then \
		echo "Git not available. Attempting to download individual files via curl..."; \
		echo "Note: This method may not work for all files. Consider installing git for better results."; \
		echo ""; \
		echo "Using git sparse checkout is recommended:"; \
		echo "  1. Install git"; \
		echo "  2. Run: make download-test-data"; \
	else \
		echo "Error: Neither git nor curl is available."; \
		echo "Please install git to download test data:"; \
		echo "  Ubuntu/Debian: apt-get install git"; \
		echo "  macOS: brew install git"; \
		echo "  Or download manually from:"; \
		echo "  https://community.opengroup.org/osdu/platform/domain-data-mgmt-services/seismic/open-vds/-/tree/master/tests/VDS"; \
		exit 1; \
	fi

# Clean test data
clean-test-data:
	@echo "Removing test data..."
	@rm -rf test-data/open-vds
	@echo "Test data removed."

# Clean
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

clean-all: clean clean-test-data
	@echo "All artifacts cleaned."

# Install development tools
install-tools:
	@echo "Installing development tools..."
	rustup component add rustfmt
	rustup component add clippy
	@if ! command -v cargo-audit >/dev/null 2>&1; then \
		echo "Installing cargo-audit..."; \
		cargo install cargo-audit; \
	fi
	@if ! command -v cargo-outdated >/dev/null 2>&1; then \
		echo "Installing cargo-outdated..."; \
		cargo install cargo-outdated; \
	fi
	@echo "Development tools installed!"

# CI checks (run all checks that CI would run)
ci-local:
	@echo "=== Running local CI checks ==="
	@echo ""
	@echo "1/7: Format check..."
	@make fmt-check
	@echo ""
	@echo "2/7: Clippy lint..."
	@make lint
	@echo ""
	@echo "3/7: Build check..."
	@make build
	@echo ""
	@echo "4/7: Tests..."
	@make test-all
	@echo ""
	@echo "5/7: Documentation..."
	@make doc
	@echo ""
	@echo "6/7: Examples..."
	@make examples
	@echo ""
	@echo "7/7: MSRV check..."
	@make check-msrv
	@echo ""
	@echo "=== All CI checks passed! ==="

# Build targets for different platforms (optional)
build-linux-musl:
	@echo "Building for x86_64-unknown-linux-musl..."
	@rustup target add x86_64-unknown-linux-musl
	cargo build --target x86_64-unknown-linux-musl --no-default-features --release

build-wasm:
	@echo "Building for wasm32-unknown-unknown..."
	@rustup target add wasm32-unknown-unknown
	cargo build --target wasm32-unknown-unknown --no-default-features

# Audit security vulnerabilities
audit:
	@echo "Checking for security vulnerabilities..."
	@if command -v cargo-audit >/dev/null 2>&1; then \
		cargo audit; \
	else \
		echo "cargo-audit not installed. Run 'make install-tools' first."; \
		exit 1; \
	fi

# Check for outdated dependencies
outdated:
	@echo "Checking for outdated dependencies..."
	@if command -v cargo-outdated >/dev/null 2>&1; then \
		cargo outdated; \
	else \
		echo "cargo-outdated not installed. Run 'make install-tools' first."; \
		exit 1; \
	fi

# Update dependencies
update:
	@echo "Updating dependencies..."
	cargo update

# Verify the project can build with specific feature flag removed
verify-no-cloud-features:
	@echo "Verifying cloud features are removed..."
	@if cargo build --features aws 2>&1 | grep -q "does not contain this feature"; then \
		echo "✓ Cloud features successfully removed"; \
	else \
		echo "✗ Cloud features still present"; \
		exit 1; \
	fi
	@if cargo build --features azure 2>&1 | grep -q "does not contain this feature"; then \
		echo "✓ Azure feature successfully removed"; \
	else \
		echo "✗ Azure feature still present"; \
		exit 1; \
	fi
	@if cargo build --features gcs 2>&1 | grep -q "does not contain this feature"; then \
		echo "✓ GCS feature successfully removed"; \
	else \
		echo "✗ GCS feature still present"; \
		exit 1; \
	fi

# Quick development cycle
dev: fmt lint test
	@echo "Development cycle complete!"

# Pre-commit hook
pre-commit: fmt lint test
	@echo "Pre-commit checks passed!"

# Coverage (requires tarpaulin)
coverage:
	@echo "Generating code coverage..."
	@if command -v cargo-tarpaulin >/dev/null 2>&1; then \
		cargo tarpaulin --no-default-features --out Html --output-dir coverage; \
		echo "Coverage report generated in coverage/"; \
	else \
		echo "cargo-tarpaulin not installed."; \
		echo "Install with: cargo install cargo-tarpaulin"; \
		exit 1; \
	fi

# Benchmarks (when available)
bench:
	@echo "Running benchmarks..."
	@if [ -d "benches" ]; then \
		cargo bench; \
	else \
		echo "No benchmarks found. Create benches/ directory with benchmark files."; \
	fi

# Watch for changes and rebuild (requires cargo-watch)
watch:
	@echo "Watching for changes..."
	@if command -v cargo-watch >/dev/null 2>&1; then \
		cargo watch -x 'check --no-default-features' -x 'test --no-default-features'; \
	else \
		echo "cargo-watch not installed."; \
		echo "Install with: cargo install cargo-watch"; \
		exit 1; \
	fi

# Run specific example
run-example-%:
	@echo "Running example: $*..."
	cargo run --example $*

# Show project statistics
stats:
	@echo "Project Statistics"
	@echo "=================="
	@echo ""
	@echo "Lines of code:"
	@find src -name '*.rs' -exec wc -l {} + | tail -1
	@echo ""
	@echo "Number of modules:"
	@find src -name '*.rs' | wc -l
	@echo ""
	@echo "Number of tests:"
	@grep -r "#\[test\]" src | wc -l
	@echo ""
	@echo "Dependencies:"
	@cargo tree --depth 1 | grep -v "^openvds" | wc -l

# Show version info
version:
	@echo "Rust version:"
	@rustc --version
	@echo ""
	@echo "Cargo version:"
	@cargo --version
	@echo ""
	@echo "openvds-rs version:"
	@cargo pkgid | cut -d# -f2

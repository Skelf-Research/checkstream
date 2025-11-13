.PHONY: help build test clean run dev fmt lint check docker bench install

# Default target
help:
	@echo "CheckStream - Makefile Commands"
	@echo ""
	@echo "Development:"
	@echo "  make build        - Build all crates in release mode"
	@echo "  make dev          - Build and run in development mode"
	@echo "  make test         - Run all tests"
	@echo "  make bench        - Run benchmarks"
	@echo "  make check        - Quick compile check"
	@echo ""
	@echo "Code Quality:"
	@echo "  make fmt          - Format code with rustfmt"
	@echo "  make lint         - Run clippy linter"
	@echo "  make audit        - Security audit with cargo-audit"
	@echo ""
	@echo "Deployment:"
	@echo "  make docker       - Build Docker image"
	@echo "  make docker-run   - Run with Docker Compose"
	@echo "  make install      - Install binary to ~/.cargo/bin"
	@echo ""
	@echo "Utilities:"
	@echo "  make clean        - Remove build artifacts"
	@echo "  make docs         - Generate and open documentation"

# Build release binary
build:
	@echo "Building release binary..."
	cargo build --release --all
	@echo "Binary available at: target/release/checkstream-proxy"

# Development mode with auto-reload (requires cargo-watch)
dev:
	@echo "Starting development server with auto-reload..."
	@command -v cargo-watch >/dev/null 2>&1 || { echo "Installing cargo-watch..."; cargo install cargo-watch; }
	cargo watch -x "run --bin checkstream-proxy -- --verbose"

# Run tests
test:
	@echo "Running tests..."
	cargo test --all --verbose

# Run tests with coverage (requires cargo-tarpaulin)
test-coverage:
	@echo "Running tests with coverage..."
	@command -v cargo-tarpaulin >/dev/null 2>&1 || { echo "Installing cargo-tarpaulin..."; cargo install cargo-tarpaulin; }
	cargo tarpaulin --all --out Html

# Run benchmarks
bench:
	@echo "Running benchmarks..."
	cargo bench --all

# Quick check without full build
check:
	@echo "Checking code..."
	cargo check --all

# Format code
fmt:
	@echo "Formatting code..."
	cargo fmt --all

# Check formatting
fmt-check:
	@echo "Checking code format..."
	cargo fmt --all -- --check

# Lint with clippy
lint:
	@echo "Running clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

# Security audit
audit:
	@echo "Running security audit..."
	@command -v cargo-audit >/dev/null 2>&1 || { echo "Installing cargo-audit..."; cargo install cargo-audit; }
	cargo audit

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf target/

# Build Docker image
docker:
	@echo "Building Docker image..."
	docker build -t checkstream:latest .

# Run with Docker Compose
docker-run:
	@echo "Starting services with Docker Compose..."
	docker-compose up

# Stop Docker Compose services
docker-stop:
	@echo "Stopping Docker Compose services..."
	docker-compose down

# Install binary
install:
	@echo "Installing checkstream-proxy..."
	cargo install --path crates/checkstream-proxy

# Generate and open documentation
docs:
	@echo "Generating documentation..."
	cargo doc --all --no-deps --open

# Run the proxy (development)
run:
	@echo "Starting CheckStream proxy..."
	cargo run --bin checkstream-proxy -- \
		--backend https://api.openai.com/v1 \
		--policy ./policies/default.yaml \
		--port 8080 \
		--verbose

# Run the proxy (release)
run-release:
	@echo "Starting CheckStream proxy (release)..."
	./target/release/checkstream-proxy \
		--backend https://api.openai.com/v1 \
		--policy ./policies/default.yaml \
		--port 8080

# Pre-commit checks (format, lint, test)
pre-commit: fmt lint test
	@echo "All pre-commit checks passed!"

# CI pipeline (what GitHub Actions runs)
ci: fmt-check lint test
	@echo "CI checks passed!"

# Setup development environment
setup:
	@echo "Setting up development environment..."
	@command -v rustup >/dev/null 2>&1 || { echo "Please install Rust from https://rustup.rs/"; exit 1; }
	rustup update
	rustup component add rustfmt clippy
	@echo "Installing dev tools..."
	cargo install cargo-watch cargo-audit cargo-tarpaulin || true
	@echo "Setup complete!"

# Generate a sample config file
config:
	@echo "Generating sample config.yaml..."
	@cat > config.local.yaml <<EOF
# CheckStream Local Configuration
backend_url: "https://api.openai.com/v1"
policy_path: "./policies/default.yaml"
token_holdback: 10
max_buffer_capacity: 1000
telemetry:
  enabled: true
  mode: aggregate
EOF
	@echo "Created config.local.yaml"

# Show project statistics
stats:
	@echo "Project Statistics:"
	@echo ""
	@echo "Lines of Rust code:"
	@find crates -name "*.rs" | xargs wc -l | tail -1
	@echo ""
	@echo "Number of crates:"
	@ls -d crates/*/ | wc -l
	@echo ""
	@echo "Dependencies:"
	@cargo tree --depth 1 | wc -l

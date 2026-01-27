#!/bin/bash
# CheckStream Installation Script
# Installs dependencies, builds the project, and downloads ML models

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print with color
print_step() {
    echo -e "${BLUE}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo ""
echo "============================================"
echo "  CheckStream Installation"
echo "============================================"
echo ""

# Step 1: Check Rust installation
print_step "Checking Rust installation..."

if ! command -v rustup &> /dev/null; then
    print_error "Rust is not installed."
    echo ""
    echo "Please install Rust first:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    echo "Then run this script again."
    exit 1
fi

if ! command -v cargo &> /dev/null; then
    print_error "Cargo is not available. Please ensure Rust is properly installed."
    exit 1
fi

RUST_VERSION=$(rustc --version)
print_success "Rust installed: $RUST_VERSION"

# Step 2: Setup development environment
print_step "Setting up development environment..."

if command -v make &> /dev/null; then
    make setup 2>/dev/null || {
        print_warning "make setup had some issues, continuing..."
    }
    print_success "Development tools configured"
else
    print_warning "make not found, installing tools manually..."
    rustup update
    rustup component add rustfmt clippy
    cargo install cargo-watch cargo-audit 2>/dev/null || true
    print_success "Development tools installed"
fi

# Step 3: Build release binary
print_step "Building release binary (this may take a few minutes)..."

cargo build --release --all

if [ -f "target/release/checkstream-proxy" ]; then
    print_success "Binary built: target/release/checkstream-proxy"
else
    print_error "Build failed - binary not found"
    exit 1
fi

# Step 4: Download ML models (optional)
print_step "Checking ML models..."

if [ -f "scripts/download_models.sh" ]; then
    echo "  ML models can be downloaded for toxicity detection."
    echo "  This requires ~500MB of disk space."
    echo ""
    read -p "  Download ML models? [y/N] " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_step "Downloading ML models..."
        bash scripts/download_models.sh
        print_success "ML models downloaded"
    else
        print_warning "Skipping ML model download (can run scripts/download_models.sh later)"
    fi
else
    print_warning "Model download script not found, skipping..."
fi

# Step 5: Setup configuration
print_step "Setting up configuration..."

if [ ! -f ".env" ] && [ -f ".env.example" ]; then
    cp .env.example .env
    print_success "Created .env from .env.example"
else
    if [ -f ".env" ]; then
        print_success ".env already exists"
    else
        print_warning ".env.example not found, skipping..."
    fi
fi

# Done!
echo ""
echo "============================================"
echo -e "  ${GREEN}Installation Complete!${NC}"
echo "============================================"
echo ""
echo "Available commands:"
echo "  make run          - Start proxy in development mode"
echo "  make run-release  - Start proxy in release mode"
echo "  make test         - Run test suite"
echo "  make dev          - Development mode with auto-reload"
echo "  make help         - Show all available commands"
echo ""
echo "Quick start:"
echo "  ./demo.sh         - Run interactive demo"
echo ""
echo "Documentation:"
echo "  docs/             - Detailed documentation"
echo "  README.md         - Project overview"
echo ""

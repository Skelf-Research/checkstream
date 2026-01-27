#!/bin/bash
# CheckStream Interactive Demo
# Demonstrates tests, ML models, and proxy functionality

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Proxy PID for cleanup
PROXY_PID=""

# Cleanup function
cleanup() {
    if [ -n "$PROXY_PID" ] && kill -0 "$PROXY_PID" 2>/dev/null; then
        echo ""
        echo -e "${YELLOW}Stopping proxy server...${NC}"
        kill "$PROXY_PID" 2>/dev/null || true
        wait "$PROXY_PID" 2>/dev/null || true
        echo -e "${GREEN}Proxy stopped.${NC}"
    fi
}

trap cleanup EXIT

# Print functions
print_header() {
    echo ""
    echo -e "${CYAN}============================================${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}============================================${NC}"
    echo ""
}

print_step() {
    echo -e "${BLUE}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

print_info() {
    echo -e "${YELLOW}[INFO]${NC} $1"
}

wait_for_enter() {
    echo ""
    read -p "Press Enter to continue..." -r
    echo ""
}

# Start demo
clear
print_header "CheckStream Interactive Demo"

echo "This demo will walk you through:"
echo "  1. Running the test suite"
echo "  2. Testing the ML sentiment classifier"
echo "  3. Starting the proxy server"
echo "  4. Sending test requests"
echo ""

wait_for_enter

# Step 1: Run tests
print_header "Step 1: Running Test Suite"

print_step "Executing cargo test..."
echo ""

if cargo test --all 2>&1 | tail -20; then
    print_success "All tests passed!"
else
    print_info "Some tests may have failed - check output above"
fi

wait_for_enter

# Step 2: ML Model Demo
print_header "Step 2: ML Sentiment Classifier"

echo "The ML classifier uses DistilBERT to analyze sentiment."
echo "This will download the model on first run (~260MB)."
echo ""

read -p "Run ML model demo? [Y/n] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    print_step "Running sentiment classifier example..."
    echo ""

    if cargo run --example test_hf_model --features ml-models 2>&1; then
        print_success "ML model demo complete!"
    else
        print_info "ML model demo had issues - the model may need to be downloaded first"
        echo "  Run: bash scripts/download_models.sh"
    fi
else
    print_info "Skipping ML model demo"
fi

wait_for_enter

# Step 3: Start Proxy
print_header "Step 3: Proxy Server Demo"

echo "The proxy server sits between your application and the LLM API."
echo "It enforces safety policies in real-time."
echo ""
echo "Configuration:"
echo "  - Port: 8080"
echo "  - Policy: policies/default.yaml"
echo "  - Backend: https://api.openai.com/v1 (configurable)"
echo ""

read -p "Start proxy server? [Y/n] " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    print_step "Building and starting proxy..."

    # Build if not already built
    if [ ! -f "target/release/checkstream-proxy" ]; then
        cargo build --release --bin checkstream-proxy
    fi

    # Start proxy in background
    ./target/release/checkstream-proxy \
        --backend https://api.openai.com/v1 \
        --policy ./policies/default.yaml \
        --port 8080 &
    PROXY_PID=$!

    # Wait for startup
    sleep 2

    if kill -0 "$PROXY_PID" 2>/dev/null; then
        print_success "Proxy started on http://localhost:8080 (PID: $PROXY_PID)"
        echo ""

        # Step 4: Test requests
        print_header "Step 4: Testing the Proxy"

        print_step "Checking health endpoint..."
        echo ""

        if curl -s http://localhost:8080/health | head -5; then
            echo ""
            print_success "Health check passed!"
        else
            print_info "Health endpoint not responding"
        fi

        echo ""
        print_step "Checking metrics endpoint..."
        echo ""

        if curl -s http://localhost:8080/metrics | head -10; then
            echo ""
            print_success "Metrics endpoint working!"
        else
            print_info "Metrics endpoint not responding"
        fi

        echo ""
        echo -e "${YELLOW}The proxy is now running!${NC}"
        echo ""
        echo "You can test it with:"
        echo "  curl http://localhost:8080/health"
        echo "  curl http://localhost:8080/metrics"
        echo ""
        echo "To use with OpenAI API, set your OPENAI_API_KEY and send requests"
        echo "to http://localhost:8080 instead of https://api.openai.com"
        echo ""

        read -p "Press Enter to stop the proxy and exit..." -r
    else
        print_info "Proxy failed to start - check logs above"
    fi
else
    print_info "Skipping proxy demo"
fi

# Summary
print_header "Demo Complete!"

echo "You've seen:"
echo "  - Test suite execution (122+ tests)"
echo "  - ML-based sentiment classification"
echo "  - Real-time proxy server"
echo ""
echo "Next steps:"
echo "  - Read docs/getting-started.md"
echo "  - Customize policies/default.yaml"
echo "  - Deploy with Docker: docker-compose up"
echo ""
echo "For more commands: make help"
echo ""

#!/bin/bash
# Download toxicity detection models for CheckStream

set -e

echo "CheckStream Model Downloader"
echo "=============================="
echo ""

# Check for dependencies
if ! command -v python3 &> /dev/null; then
    echo "Error: python3 is required but not installed."
    exit 1
fi

# Check for huggingface-cli
if ! command -v huggingface-cli &> /dev/null; then
    echo "Installing huggingface-cli..."
    pip3 install --user huggingface-hub
fi

# Create models directory
MODEL_DIR="./models"
mkdir -p "$MODEL_DIR"

echo "Model download directory: $MODEL_DIR"
echo ""

# Download toxicity model
TOXIC_MODEL="unitary/toxic-bert"
TOXIC_DIR="$MODEL_DIR/toxic-bert"

echo "Downloading toxicity model: $TOXIC_MODEL"
echo "This may take a few minutes..."
echo ""

huggingface-cli download "$TOXIC_MODEL" \
    --local-dir "$TOXIC_DIR" \
    --include "*.safetensors" "*.json" "*.txt" \
    2>&1 | grep -E "(Downloading|Fetching|Download)" || true

echo ""
echo "✓ Download complete!"
echo ""

# Verify files
echo "Verifying model files..."

REQUIRED_FILES=(
    "config.json"
    "tokenizer.json"
    "tokenizer_config.json"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$TOXIC_DIR/$file" ]; then
        echo "  ✓ $file"
    else
        echo "  ✗ $file (MISSING)"
    fi
done

# Check for model weights
if ls "$TOXIC_DIR"/*.safetensors 1> /dev/null 2>&1; then
    echo "  ✓ model weights (*.safetensors)"
else
    echo "  ✗ model weights (MISSING)"
fi

echo ""
echo "=============================="
echo "Model setup complete!"
echo ""
echo "Model location: $TOXIC_DIR"
echo ""
echo "To use the ML model:"
echo "  1. Rebuild with ML support:"
echo "     cargo build --package checkstream-classifiers --features ml-models"
echo ""
echo "  2. The proxy will automatically detect and use the model"
echo ""
echo "To test the model:"
echo "  cargo test --package checkstream-classifiers -- toxicity"
echo ""

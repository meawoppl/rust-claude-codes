#!/bin/bash
# Script to build all examples and check they compile

set -e

echo "Building all examples..."
echo ""

FAILED=0
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLES_DIR="$SCRIPT_DIR/examples"

# Check if examples directory exists
if [ ! -d "$EXAMPLES_DIR" ]; then
    echo "Error: examples directory not found"
    exit 1
fi

# Build each example
for example in "$EXAMPLES_DIR"/*.rs; do
    if [ -f "$example" ]; then
        example_name=$(basename "$example" .rs)
        echo "Building example: $example_name"
        
        if cargo build -p claude-codes --example "$example_name" 2>&1 | grep -q "error"; then
            echo "  ❌ Failed to build $example_name"
            FAILED=1
        else
            echo "  ✅ Successfully built $example_name"
        fi
    fi
done

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "Some examples failed to build"
    exit 1
else
    echo "All examples built successfully!"
fi
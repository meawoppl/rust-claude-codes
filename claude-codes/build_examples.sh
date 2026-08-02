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
        
        # Use cargo's exit code — grepping the output for "error" used to
        # false-positive on crate names like `thiserror` whose compile lines
        # contain the substring.
        # --all-features so examples gated on non-default features
        # (required-features in Cargo.toml, e.g. `auth`) build too.
        if cargo build -p claude-codes --all-features --example "$example_name" >/dev/null 2>&1; then
            echo "  ✅ Successfully built $example_name"
        else
            echo "  ❌ Failed to build $example_name"
            FAILED=1
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
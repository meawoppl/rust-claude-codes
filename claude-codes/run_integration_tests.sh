#!/bin/bash

# Integration test runner for claude-codes
# This script runs integration tests that interact with real Claude services

set -e

echo "🧪 Claude Codes Integration Test Runner"
echo "======================================="
echo ""

# Check if Claude CLI is installed
if ! command -v claude &> /dev/null; then
    echo "❌ Error: Claude CLI is not installed"
    echo ""
    echo "Please install it with:"
    echo "  npm install -g @anthropic-ai/claude-cli"
    echo ""
    echo "Then authenticate with:"
    echo "  claude login"
    exit 1
fi

# Get Claude version
CLAUDE_VERSION=$(claude --version 2>/dev/null || echo "unknown")
echo "✅ Claude CLI found: $CLAUDE_VERSION"
echo ""

# Parse command line arguments
VERBOSE=""
SPECIFIC_TEST=""
NOCAPTURE=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE="--verbose"
            NOCAPTURE="-- --nocapture"
            shift
            ;;
        -t|--test)
            SPECIFIC_TEST="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -v, --verbose    Show detailed output"
            echo "  -t, --test NAME  Run specific test"
            echo "  -h, --help       Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                     # Run all integration tests"
            echo "  $0 -v                  # Run with verbose output"
            echo "  $0 -t test_cli_version # Run specific test"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

# Build the test command
TEST_CMD="cargo test --features integration-tests"

if [ -n "$SPECIFIC_TEST" ]; then
    TEST_CMD="$TEST_CMD $SPECIFIC_TEST"
fi

if [ -n "$VERBOSE" ]; then
    TEST_CMD="$TEST_CMD $VERBOSE"
fi

if [ -n "$NOCAPTURE" ]; then
    TEST_CMD="$TEST_CMD $NOCAPTURE"
fi

# Run the tests
echo "🚀 Running integration tests..."
echo "Command: $TEST_CMD"
echo ""

if $TEST_CMD; then
    echo ""
    echo "✅ All integration tests passed!"
else
    echo ""
    echo "❌ Some integration tests failed"
    echo ""
    echo "Troubleshooting:"
    echo "1. Ensure Claude CLI is authenticated: claude login"
    echo "2. Check network connectivity"
    echo "3. Run with -v flag for more details"
    exit 1
fi
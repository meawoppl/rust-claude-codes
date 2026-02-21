#!/bin/bash

# Script to set up git pre-commit hooks for the project
# This will prevent direct commits to main and ensure code quality

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Setting up git hooks for rust-claude-codes...${NC}"
echo

# Create the pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash

# Pre-commit hook for rust-claude-codes
# Prevents direct commits to main branch and ensures code quality

set -e

# Colors for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

# Get the current branch name
BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Check if we're on main branch
if [ "$BRANCH" = "main" ]; then
    echo -e "${RED}✗ Direct commits to main branch are not allowed!${NC}"
    echo -e "${YELLOW}Please create a feature branch:${NC}"
    echo -e "  git checkout -b feature/your-feature-name"
    echo -e "  git commit ..."
    echo -e "  git push origin feature/your-feature-name"
    echo -e "  Then create a pull request on GitHub"
    echo
    echo -e "${YELLOW}To bypass this check (not recommended):${NC}"
    echo -e "  git commit --no-verify"
    exit 1
fi

echo -e "${GREEN}✓ Not on main branch (current: $BRANCH)${NC}"

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗ cargo is not installed or not in PATH${NC}"
    exit 1
fi

# Run cargo fmt check
echo -e "${YELLOW}Running cargo fmt check...${NC}"
if ! cargo fmt --all -- --check; then
    echo -e "${RED}✗ Code is not properly formatted!${NC}"
    echo -e "${YELLOW}Run 'cargo fmt --all' to fix formatting issues${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Code formatting check passed${NC}"

# Run cargo clippy
echo -e "${YELLOW}Running cargo clippy...${NC}"
if ! cargo clippy --all-targets --all-features -- -D warnings 2>&1; then
    echo -e "${RED}✗ Clippy found issues!${NC}"
    echo -e "${YELLOW}Fix the issues reported above before committing${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Clippy check passed${NC}"

# Check JSON formatting in test cases
if [ -d "test_cases/failed_deserializations" ]; then
    echo -e "${YELLOW}Checking JSON test case formatting...${NC}"
    
    # Check if jq is installed
    if command -v jq &> /dev/null; then
        FAILED=0
        for file in test_cases/failed_deserializations/*.json; do
            if [ -f "$file" ]; then
                # Create a formatted version
                if jq '.' "$file" > /tmp/formatted.json 2>/dev/null; then
                    # Check if the file is already formatted
                    if ! cmp -s "$file" /tmp/formatted.json; then
                        filename=$(basename "$file")
                        echo -e "${RED}✗ $filename is not properly formatted${NC}"
                        FAILED=1
                    fi
                else
                    filename=$(basename "$file")
                    echo -e "${RED}✗ $filename contains invalid JSON${NC}"
                    FAILED=1
                fi
            fi
        done
        
        if [ "$FAILED" -eq 1 ]; then
            echo -e "${YELLOW}Run './format_test_cases.sh' to fix JSON formatting${NC}"
            exit 1
        fi
        echo -e "${GREEN}✓ JSON formatting check passed${NC}"
    else
        echo -e "${YELLOW}⚠ jq not installed, skipping JSON format check${NC}"
    fi
fi

# Run tests (optional - comment out if tests take too long)
echo -e "${YELLOW}Running tests...${NC}"
if ! cargo test --quiet; then
    echo -e "${RED}✗ Tests failed!${NC}"
    echo -e "${YELLOW}Fix failing tests before committing${NC}"
    exit 1
fi
echo -e "${GREEN}✓ All tests passed${NC}"

echo -e "${GREEN}✓ All pre-commit checks passed!${NC}"
EOF

# Make the hook executable
chmod +x .git/hooks/pre-commit

echo -e "${GREEN}✓ Pre-commit hook installed successfully!${NC}"
echo
echo "The pre-commit hook will:"
echo "  • Prevent direct commits to the main branch"
echo "  • Check code formatting with 'cargo fmt'"
echo "  • Run 'cargo clippy' to catch common issues"
echo "  • Verify JSON test cases are formatted (if jq is installed)"
echo "  • Run all tests"
echo
echo -e "${YELLOW}Note: You can bypass the hook with 'git commit --no-verify' (not recommended)${NC}"
echo

# Create a post-checkout hook to remind about branch protection
cat > .git/hooks/post-checkout << 'EOF'
#!/bin/bash

# Post-checkout hook to remind about branch protection

BRANCH=$(git rev-parse --abbrev-ref HEAD)

if [ "$BRANCH" = "main" ]; then
    echo
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║ ⚠️  You are now on the main branch                          ║"
    echo "║                                                            ║"
    echo "║ Remember: Direct commits to main are blocked!             ║"
    echo "║                                                            ║"
    echo "║ To make changes:                                          ║"
    echo "║   git checkout -b feature/your-feature-name               ║"
    echo "║                                                            ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo
fi
EOF

chmod +x .git/hooks/post-checkout

echo -e "${GREEN}✓ Post-checkout hook installed successfully!${NC}"
echo

# Test the current branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" = "main" ]; then
    echo -e "${YELLOW}⚠️  You are currently on the main branch${NC}"
    echo -e "${YELLOW}   The pre-commit hook will prevent commits here${NC}"
    echo -e "${YELLOW}   Create a feature branch for your changes:${NC}"
    echo -e "${BLUE}   git checkout -b feature/your-feature-name${NC}"
else
    echo -e "${GREEN}✓ You are on branch: $CURRENT_BRANCH${NC}"
    echo -e "${GREEN}  You can commit normally on this branch${NC}"
fi

echo
echo -e "${BLUE}Setup complete! Your repository now has commit quality checks.${NC}"
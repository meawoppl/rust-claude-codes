#!/bin/bash

# Script to pretty-format all JSON test case files
# Usage: ./format_test_cases.sh

set -e

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo -e "${RED}Error: jq is not installed${NC}"
    echo "Please install jq to format JSON files:"
    echo "  Ubuntu/Debian: sudo apt-get install jq"
    echo "  macOS: brew install jq"
    echo "  Fedora: sudo dnf install jq"
    exit 1
fi

# Directory containing test cases
TEST_DIR="test_cases/failed_deserializations"

# Check if directory exists
if [ ! -d "$TEST_DIR" ]; then
    echo -e "${YELLOW}Warning: Directory $TEST_DIR does not exist${NC}"
    exit 0
fi

# Count JSON files
JSON_COUNT=$(find "$TEST_DIR" -name "*.json" -type f 2>/dev/null | wc -l)

if [ "$JSON_COUNT" -eq 0 ]; then
    echo -e "${YELLOW}No JSON files found in $TEST_DIR${NC}"
    exit 0
fi

echo -e "${GREEN}Found $JSON_COUNT JSON file(s) to format${NC}"
echo

# Process each JSON file
FORMATTED=0
ERRORS=0

for file in "$TEST_DIR"/*.json; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        echo -n "Formatting $filename... "
        
        # Create a temporary file
        temp_file=$(mktemp)
        
        # Try to format the JSON file
        if jq '.' "$file" > "$temp_file" 2>/dev/null; then
            # Check if the file actually changed
            if ! cmp -s "$file" "$temp_file"; then
                mv "$temp_file" "$file"
                echo -e "${GREEN}✓${NC}"
                ((FORMATTED++))
            else
                rm "$temp_file"
                echo "already formatted"
            fi
        else
            rm "$temp_file"
            echo -e "${RED}✗ (invalid JSON)${NC}"
            
            # Try to show what's wrong
            echo -e "  ${YELLOW}Error details:${NC}"
            jq '.' "$file" 2>&1 | head -n 3 | sed 's/^/    /'
            ((ERRORS++))
        fi
    fi
done

echo
echo "Summary:"
echo -e "  ${GREEN}Formatted: $FORMATTED file(s)${NC}"
if [ "$ERRORS" -gt 0 ]; then
    echo -e "  ${RED}Errors: $ERRORS file(s)${NC}"
fi

# Special handling for our test case format
# These files have a specific structure with raw_json and pretty_json fields
echo
echo "Updating pretty_json fields in test cases..."

for file in "$TEST_DIR"/*.json; do
    if [ -f "$file" ]; then
        filename=$(basename "$file")
        
        # Check if it's a test case file (has raw_json field)
        if jq -e '.raw_json' "$file" > /dev/null 2>&1; then
            echo -n "Updating $filename... "
            
            # Create a temporary file
            temp_file=$(mktemp)
            
            # Update the pretty_json field with formatted version of raw_json
            if jq '.pretty_json = (.raw_json | fromjson | tojson(2))' "$file" > "$temp_file" 2>/dev/null; then
                mv "$temp_file" "$file"
                
                # Format the entire file again
                temp_file=$(mktemp)
                jq '.' "$file" > "$temp_file" && mv "$temp_file" "$file"
                
                echo -e "${GREEN}✓${NC}"
            else
                rm -f "$temp_file"
                echo -e "${YELLOW}skipped (not a test case format)${NC}"
            fi
        fi
    fi
done

echo
echo -e "${GREEN}Done!${NC}"
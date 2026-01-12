#!/bin/bash
# Run LLM integration tests with logging enabled.
#
# Usage:
#   ./scripts/test-integration.sh              # Run all LLM integration tests
#   ./scripts/test-integration.sh narrative    # Run tests matching "narrative"
#   ./scripts/test-integration.sh --no-log     # Run without logging
#
# Environment variables:
#   LLM_TEST_LOG_DIR  - Directory for log files (default: ./llm_test_logs)
#   OLLAMA_BASE_URL   - Ollama API URL (default: http://localhost:11434)
#   OLLAMA_MODEL      - Model to use (default: gpt-oss:20b)
#
# Requirements:
#   - Ollama must be running with the configured model

set -e

# Parse arguments
ENABLE_LOG=1
TEST_FILTER=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-log)
            ENABLE_LOG=0
            shift
            ;;
        *)
            TEST_FILTER="$1"
            shift
            ;;
    esac
done

# Set up environment
export LLM_TEST_LOG="${ENABLE_LOG}"

# Check if Ollama is available
if ! curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "Warning: Ollama does not appear to be running at localhost:11434"
    echo "Integration tests will fail without Ollama."
    echo ""
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Build test filter
if [[ -n "$TEST_FILTER" ]]; then
    echo "Running LLM integration tests matching: $TEST_FILTER"
    FILTER_ARG="$TEST_FILTER"
else
    echo "Running all LLM integration tests"
    FILTER_ARG=""
fi

if [[ "$ENABLE_LOG" == "1" ]]; then
    LOG_DIR="${LLM_TEST_LOG_DIR:-./llm_test_logs}"
    echo "Logging enabled: $LOG_DIR"
else
    echo "Logging disabled"
fi

echo ""

# Run the tests
# Using --test-threads=1 to avoid overwhelming Ollama with parallel requests
cargo test -p wrldbldr-engine --lib ${FILTER_ARG:+"$FILTER_ARG"} -- \
    --ignored \
    --nocapture \
    --test-threads=1

echo ""
echo "Tests completed!"

if [[ "$ENABLE_LOG" == "1" ]]; then
    LOG_DIR="${LLM_TEST_LOG_DIR:-./llm_test_logs}"
    if [[ -d "$LOG_DIR" ]]; then
        LOG_COUNT=$(find "$LOG_DIR" -name "*.md" -type f 2>/dev/null | wc -l | tr -d ' ')
        echo "Log files: $LOG_COUNT files in $LOG_DIR"
    fi
fi

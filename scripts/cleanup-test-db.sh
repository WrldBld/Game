#!/bin/bash
# Clean up all data from the E2E test Neo4j database.
#
# Usage:
#   ./scripts/cleanup-test-db.sh           # Clean up all test data
#   ./scripts/cleanup-test-db.sh --dry-run # Show what would be deleted without deleting
#
# This script:
#   1. Checks if the test container (wrldbldr-test-neo4j) is running
#   2. Verifies the Neo4j connection is available
#   3. Shows current node/relationship counts
#   4. Deletes all data from the database
#
# Requirements:
#   - Docker must be running
#   - The test container must be running (start with: task e2e)

set -e

# Configuration (must match neo4j_test_harness.rs)
CONTAINER_NAME="wrldbldr-test-neo4j"
BOLT_PORT=17687
NEO4J_PASSWORD="testpassword"
BOLT_URI="bolt://localhost:${BOLT_PORT}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
DRY_RUN=0
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--dry-run]"
            echo ""
            echo "Options:"
            echo "  --dry-run    Show what would be deleted without actually deleting"
            echo "  -h, --help   Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo "======================================"
echo "  E2E Test Database Cleanup"
echo "======================================"
echo ""

# Step 1: Check if Docker is available
echo -n "Checking Docker... "
if ! command -v docker &> /dev/null; then
    echo -e "${RED}FAILED${NC}"
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi
echo -e "${GREEN}OK${NC}"

# Step 2: Check if test container is running
echo -n "Checking test container '${CONTAINER_NAME}'... "
CONTAINER_STATUS=$(docker ps --filter "name=${CONTAINER_NAME}" --format "{{.Status}}" 2>/dev/null || echo "")

if [[ -z "$CONTAINER_STATUS" ]]; then
    echo -e "${RED}NOT RUNNING${NC}"
    echo ""
    echo "The test container is not running."
    echo "Start it by running: task e2e"
    echo ""
    echo "Or start just the container with:"
    echo "  docker run -d --name ${CONTAINER_NAME} \\"
    echo "    -p ${BOLT_PORT}:7687 -p 17474:7474 \\"
    echo "    -e NEO4J_AUTH=neo4j/${NEO4J_PASSWORD} \\"
    echo "    neo4j:5.26.0-community"
    exit 1
fi
echo -e "${GREEN}RUNNING${NC} ($CONTAINER_STATUS)"

# Step 3: Check if Neo4j is accepting connections
echo -n "Checking Neo4j connection... "

# Use cypher-shell inside the container to test connectivity
CONNECTION_TEST=$(docker exec ${CONTAINER_NAME} \
    cypher-shell -u neo4j -p ${NEO4J_PASSWORD} \
    "RETURN 1 AS connected" 2>&1) || true

if [[ "$CONNECTION_TEST" != *"connected"* ]]; then
    echo -e "${RED}FAILED${NC}"
    echo ""
    echo "Neo4j is not ready to accept connections."
    echo "It may still be starting up. Wait a few seconds and try again."
    echo ""
    echo "Debug output:"
    echo "$CONNECTION_TEST"
    exit 1
fi
echo -e "${GREEN}OK${NC}"

# Step 4: Get current database statistics
echo ""
echo "Current database state:"
echo "----------------------"

# Use --format plain to avoid header decorations
NODE_COUNT=$(docker exec ${CONTAINER_NAME} \
    cypher-shell -u neo4j -p ${NEO4J_PASSWORD} --format plain \
    "MATCH (n) RETURN count(n) as count" 2>/dev/null | tail -1 | tr -d ' ')

REL_COUNT=$(docker exec ${CONTAINER_NAME} \
    cypher-shell -u neo4j -p ${NEO4J_PASSWORD} --format plain \
    "MATCH ()-[r]->() RETURN count(r) as count" 2>/dev/null | tail -1 | tr -d ' ')

WORLD_COUNT=$(docker exec ${CONTAINER_NAME} \
    cypher-shell -u neo4j -p ${NEO4J_PASSWORD} --format plain \
    "MATCH (w:World) RETURN count(w) as count" 2>/dev/null | tail -1 | tr -d ' ')

echo "  Nodes:         ${NODE_COUNT:-0}"
echo "  Relationships: ${REL_COUNT:-0}"
echo "  Worlds:        ${WORLD_COUNT:-0}"

echo ""

# Step 5: Perform cleanup (or dry-run)
if [[ "$DRY_RUN" == "1" ]]; then
    echo -e "${YELLOW}DRY RUN MODE${NC} - No changes will be made"
    echo ""
    echo "Would delete:"
    echo "  - All ${NODE_COUNT:-unknown} nodes"
    echo "  - All ${REL_COUNT:-unknown} relationships"
    echo ""
    echo "Run without --dry-run to actually delete."
    exit 0
fi

# Confirm deletion if there's data
if [[ "${NODE_COUNT:-0}" != "0" ]]; then
    echo -n "Delete all data? [y/N] "
    read -r CONFIRM
    if [[ ! "$CONFIRM" =~ ^[Yy]$ ]]; then
        echo "Cancelled."
        exit 0
    fi
fi

echo ""
echo -n "Deleting all data... "

# Delete all nodes and relationships
# Using CALL with IN TRANSACTIONS for large datasets
RESULT=$(docker exec ${CONTAINER_NAME} \
    cypher-shell -u neo4j -p ${NEO4J_PASSWORD} \
    "MATCH (n) DETACH DELETE n" 2>&1)

if [[ $? -eq 0 ]]; then
    echo -e "${GREEN}DONE${NC}"
else
    echo -e "${RED}FAILED${NC}"
    echo "Error: $RESULT"
    exit 1
fi

# Step 6: Verify cleanup
echo -n "Verifying cleanup... "

VERIFY=$(docker exec ${CONTAINER_NAME} \
    cypher-shell -u neo4j -p ${NEO4J_PASSWORD} --format plain \
    "MATCH (n) RETURN count(n) as count" 2>/dev/null | tail -1 | tr -d ' ')

if [[ "$VERIFY" == "0" ]]; then
    echo -e "${GREEN}OK${NC} (0 nodes remaining)"
else
    echo -e "${YELLOW}WARNING${NC} ($VERIFY nodes remaining)"
fi

echo ""
echo "======================================"
echo "  Cleanup complete!"
echo "======================================"

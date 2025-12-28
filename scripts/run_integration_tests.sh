#!/bin/bash
# Run PostgreSQL integration tests
set -e

# Color output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== PostgreSQL Integration Tests ===${NC}\n"

# Check if Docker container is running
if ! docker ps | grep -q rstn-postgres; then
    echo -e "${RED}Error: PostgreSQL container 'rstn-postgres' is not running${NC}"
    echo "Start it with: docker start rstn-postgres"
    exit 1
fi

# Set up test database
echo -e "${BLUE}Step 1: Setting up test database...${NC}"
bash "$(dirname "$0")/setup_test_db.sh"

# Run integration tests
echo -e "\n${BLUE}Step 2: Running integration tests...${NC}"
POSTGRES_URI="postgresql://rstn:rstn@localhost:5432/data_bridge_test" \
    uv run pytest tests/postgres/integration/ -v --tb=short -m integration

# Check exit code
if [ $? -eq 0 ]; then
    echo -e "\n${GREEN}=== Integration tests passed! ===${NC}"
else
    echo -e "\n${RED}=== Integration tests failed ===${NC}"
    exit 1
fi

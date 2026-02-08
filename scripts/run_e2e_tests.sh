#!/bin/bash
# Helper script to run end-to-end tests
# This script starts the required infrastructure and runs the E2E test suite

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}End-to-End Test Runner for label-verify-hw${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${RED}❌ .env file not found${NC}"
    echo "Please create a .env file with required configuration."
    echo "See .env.example for reference."
    exit 1
fi

# Source .env file
set -a
source .env
set +a

echo -e "${GREEN}✓${NC} Loaded environment from .env"

# Check if required services are running
echo ""
echo -e "${YELLOW}Checking required services...${NC}"

# Check PostgreSQL
if ! nc -z localhost 5432 2>/dev/null; then
    echo -e "${RED}❌ PostgreSQL not running on port 5432${NC}"
    echo "Start with: docker compose up -d postgres"
    exit 1
fi
echo -e "${GREEN}✓${NC} PostgreSQL is running"

# Check Redis
if ! nc -z localhost 6379 2>/dev/null; then
    echo -e "${RED}❌ Redis not running on port 6379${NC}"
    echo "Start with: docker compose up -d redis"
    exit 1
fi
echo -e "${GREEN}✓${NC} Redis is running"

# Run migrations
echo ""
echo -e "${YELLOW}Running database migrations...${NC}"
export PATH="$HOME/.cargo/bin:$PATH"
if ! cargo sqlx migrate run 2>/dev/null; then
    echo -e "${YELLOW}⚠${NC}  Migration warning (may already be applied)"
fi

# Check if API server is running
API_PORT=${BIND_ADDR##*:}
API_PORT=${API_PORT:-3000}

if ! nc -z localhost $API_PORT 2>/dev/null; then
    echo -e "${RED}❌ API server not running on port $API_PORT${NC}"
    echo ""
    echo "Start in a separate terminal with:"
    echo "  cargo run --bin api"
    echo ""
    read -p "Press Enter when API server is ready..."
fi

# Check health endpoint
echo ""
echo -e "${YELLOW}Checking API health...${NC}"
if curl -s http://localhost:$API_PORT/health > /dev/null; then
    echo -e "${GREEN}✓${NC} API server is healthy"
else
    echo -e "${RED}❌ API health check failed${NC}"
    echo "Make sure the API server is running: cargo run --bin api"
    exit 1
fi

# Prompt for worker
echo ""
echo -e "${YELLOW}Checking worker process...${NC}"
echo "Make sure the worker is running in a separate terminal:"
echo "  cargo run --bin worker"
echo ""
read -p "Press Enter when worker is ready (or Ctrl+C to cancel)..."

# Determine which test to run
TEST_NAME=${1:-""}

echo ""
echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}Running E2E Tests${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

if [ -z "$TEST_NAME" ]; then
    echo "Running all E2E tests..."
    echo ""
    cargo test --test e2e_test -- --ignored --nocapture --test-threads=1
else
    echo "Running specific test: $TEST_NAME"
    echo ""
    cargo test --test e2e_test "$TEST_NAME" -- --ignored --nocapture
fi

EXIT_CODE=$?

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}================================================${NC}"
    echo -e "${GREEN}✅ All tests passed!${NC}"
    echo -e "${GREEN}================================================${NC}"
else
    echo -e "${RED}================================================${NC}"
    echo -e "${RED}❌ Some tests failed${NC}"
    echo -e "${RED}================================================${NC}"
fi

exit $EXIT_CODE

#!/bin/bash
set -e

# Label Verify HW - Deployment Script
# This script automates the deployment process

echo "🚀 Label Verify HW - Deployment Script"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if .env.prod exists
if [ ! -f .env.prod ]; then
    echo -e "${RED}❌ Error: .env.prod not found${NC}"
    echo ""
    echo "Please create .env.prod from .env.prod.example:"
    echo "  cp .env.prod.example .env.prod"
    echo "  nano .env.prod  # Fill in production values"
    exit 1
fi

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Error: Docker is not installed${NC}"
    echo "Install Docker: https://docs.docker.com/get-docker/"
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker compose &> /dev/null; then
    echo -e "${RED}❌ Error: Docker Compose is not installed${NC}"
    echo "Install Docker Compose: https://docs.docker.com/compose/install/"
    exit 1
fi

# Parse arguments
COMMAND=${1:-deploy}

case $COMMAND in
    deploy)
        echo "📦 Building Docker images..."
        docker compose --env-file .env.prod build

        echo ""
        echo "🚀 Starting services..."
        docker compose --env-file .env.prod up -d

        echo ""
        echo "⏳ Waiting for services to be healthy..."
        sleep 5

        echo ""
        echo "📊 Service Status:"
        docker compose --env-file .env.prod ps

        echo ""
        echo -e "${GREEN}✅ Deployment complete!${NC}"
        echo ""
        echo "Next steps:"
        echo "  • Check logs: ./deploy.sh logs"
        echo "  • Test API: curl http://localhost:3000/health"
        echo "  • Scale workers: docker compose --env-file .env.prod up -d --scale worker=3"
        ;;

    logs)
        echo "📋 Showing logs (Ctrl+C to exit)..."
        docker compose --env-file .env.prod logs -f
        ;;

    stop)
        echo "🛑 Stopping services..."
        docker compose --env-file .env.prod down
        echo -e "${GREEN}✅ Services stopped${NC}"
        ;;

    restart)
        echo "🔄 Restarting services..."
        docker compose --env-file .env.prod restart
        echo -e "${GREEN}✅ Services restarted${NC}"
        ;;

    status)
        echo "📊 Service Status:"
        docker compose --env-file .env.prod ps
        ;;

    clean)
        echo -e "${YELLOW}⚠️  This will remove all containers and volumes (data will be lost)${NC}"
        read -p "Are you sure? (y/N) " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            docker compose --env-file .env.prod down -v
            echo -e "${GREEN}✅ Cleaned up${NC}"
        else
            echo "Cancelled"
        fi
        ;;

    scale)
        WORKERS=${2:-2}
        echo "📈 Scaling workers to $WORKERS instances..."
        docker compose --env-file .env.prod up -d --scale worker=$WORKERS
        echo -e "${GREEN}✅ Scaled to $WORKERS workers${NC}"
        ;;

    backup)
        BACKUP_DIR="./backups/$(date +%Y%m%d_%H%M%S)"
        mkdir -p "$BACKUP_DIR"

        echo "💾 Creating backup in $BACKUP_DIR..."

        # Backup PostgreSQL
        echo "  • Backing up PostgreSQL..."
        docker compose --env-file .env.prod exec -T postgres \
            pg_dump -U labelverify labelverify_prod > "$BACKUP_DIR/database.sql"

        # Backup Redis (if needed)
        echo "  • Backing up Redis..."
        docker compose --env-file .env.prod exec -T redis \
            redis-cli --no-auth-warning -a "${REDIS_PASSWORD}" SAVE

        echo -e "${GREEN}✅ Backup complete: $BACKUP_DIR${NC}"
        ;;

    test)
        echo "🧪 Testing deployment..."

        # Test health endpoint
        echo "  • Testing health endpoint..."
        if curl -f http://localhost:3000/health > /dev/null 2>&1; then
            echo -e "    ${GREEN}✓ Health check passed${NC}"
        else
            echo -e "    ${RED}✗ Health check failed${NC}"
            exit 1
        fi

        # Test database connection
        echo "  • Testing database connection..."
        if docker compose --env-file .env.prod exec -T postgres \
            pg_isready -U labelverify > /dev/null 2>&1; then
            echo -e "    ${GREEN}✓ Database connection OK${NC}"
        else
            echo -e "    ${RED}✗ Database connection failed${NC}"
            exit 1
        fi

        # Test Redis connection
        echo "  • Testing Redis connection..."
        if docker compose --env-file .env.prod exec -T redis \
            redis-cli --no-auth-warning -a "${REDIS_PASSWORD}" ping > /dev/null 2>&1; then
            echo -e "    ${GREEN}✓ Redis connection OK${NC}"
        else
            echo -e "    ${RED}✗ Redis connection failed${NC}"
            exit 1
        fi

        echo ""
        echo -e "${GREEN}✅ All tests passed!${NC}"
        ;;

    *)
        echo "Usage: ./deploy.sh [command]"
        echo ""
        echo "Commands:"
        echo "  deploy      Build and start all services (default)"
        echo "  logs        Show and follow logs"
        echo "  stop        Stop all services"
        echo "  restart     Restart all services"
        echo "  status      Show service status"
        echo "  clean       Remove all containers and volumes (WARNING: destroys data)"
        echo "  scale N     Scale workers to N instances"
        echo "  backup      Create database backup"
        echo "  test        Test deployment health"
        echo ""
        echo "Examples:"
        echo "  ./deploy.sh deploy          # Deploy application"
        echo "  ./deploy.sh logs            # View logs"
        echo "  ./deploy.sh scale 5         # Scale to 5 workers"
        echo "  ./deploy.sh backup          # Create backup"
        ;;
esac

# Cloudflare Deployment Guide

This guide covers deploying the label-verify-hw application using Cloudflare infrastructure and services.

## Architecture Considerations

Our application currently uses:
- **Rust/Axum** - Web server
- **PostgreSQL** - Job tracking database
- **Redis** - Job queue
- **Workers AI** - OCR processing (✅ already Cloudflare)
- **R2 Storage** - Image storage (✅ already Cloudflare)

## Deployment Options

### Option 1: Containerized Deployment with Cloudflare Tunnel (Recommended)

Deploy the application in containers and connect to Cloudflare's network via Cloudflare Tunnel.

**Pros**:
- Use existing codebase as-is
- Secure connection without exposing ports
- Cloudflare CDN and DDoS protection
- Full PostgreSQL and Redis support
- Easy to scale horizontally

**Cons**:
- Requires managing container infrastructure
- Need to provision PostgreSQL and Redis separately

### Option 2: Cloudflare Workers Refactor

Refactor the application to run entirely on Cloudflare Workers with D1 (SQLite) and Queues.

**Pros**:
- Fully serverless
- Auto-scaling
- No infrastructure management
- Global edge deployment

**Cons**:
- Significant refactoring required
- D1 is SQLite (not PostgreSQL)
- Different runtime constraints
- Some Rust dependencies not compatible

### Option 3: Hybrid Approach

Keep the Rust API server but use Cloudflare Workers as a frontend/proxy.

**Pros**:
- Edge caching for static content
- DDoS protection at edge
- Keep existing backend architecture

**Cons**:
- More complex architecture
- Additional Cloudflare Workers code needed

---

## Recommended: Option 1 - Containerized Deployment

This approach keeps our existing architecture and adds Cloudflare's network benefits.

### Step 1: Create Docker Containers

#### Dockerfile for API Server

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release --bin label-verify-hw

# Runtime stage
FROM debian:bookworm-slim

# Install dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/label-verify-hw /usr/local/bin/label-verify-hw

# Copy migrations
COPY migrations /migrations

# Expose port
EXPOSE 3000

# Run the binary
CMD ["label-verify-hw"]
```

#### Dockerfile for Worker

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build worker binary
RUN cargo build --release --bin worker

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/worker /usr/local/bin/worker

CMD ["worker"]
```

#### docker-compose.yml

```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15-alpine
    environment:
      POSTGRES_DB: labelverify_prod
      POSTGRES_USER: labelverify
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    volumes:
      - postgres-data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U labelverify"]
      interval: 10s
      timeout: 5s
      retries: 5

  redis:
    image: redis:7-alpine
    command: redis-server --requirepass ${REDIS_PASSWORD}
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 10s
      timeout: 5s
      retries: 5

  api:
    build:
      context: .
      dockerfile: Dockerfile.api
    ports:
      - "3000:3000"
    environment:
      BIND_ADDR: 0.0.0.0:3000
      DATABASE_URL: postgresql://labelverify:${DB_PASSWORD}@postgres:5432/labelverify_prod
      REDIS_URL: redis://:${REDIS_PASSWORD}@redis:6379
      CF_ACCOUNT_ID: ${CF_ACCOUNT_ID}
      CF_API_TOKEN: ${CF_API_TOKEN}
      R2_BUCKET: ${R2_BUCKET}
      R2_ACCESS_KEY: ${R2_ACCESS_KEY}
      R2_SECRET_KEY: ${R2_SECRET_KEY}
      R2_ENDPOINT: ${R2_ENDPOINT}
      ENCRYPTION_KEY: ${ENCRYPTION_KEY}
      AZURE_TENANT_ID: ${AZURE_TENANT_ID}
      AZURE_CLIENT_ID: ${AZURE_CLIENT_ID}
      RUST_LOG: info
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    restart: unless-stopped

  worker:
    build:
      context: .
      dockerfile: Dockerfile.worker
    environment:
      DATABASE_URL: postgresql://labelverify:${DB_PASSWORD}@postgres:5432/labelverify_prod
      REDIS_URL: redis://:${REDIS_PASSWORD}@redis:6379
      CF_ACCOUNT_ID: ${CF_ACCOUNT_ID}
      CF_API_TOKEN: ${CF_API_TOKEN}
      R2_BUCKET: ${R2_BUCKET}
      R2_ACCESS_KEY: ${R2_ACCESS_KEY}
      R2_SECRET_KEY: ${R2_SECRET_KEY}
      R2_ENDPOINT: ${R2_ENDPOINT}
      ENCRYPTION_KEY: ${ENCRYPTION_KEY}
      RUST_LOG: info
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy
    restart: unless-stopped
    # Scale workers as needed
    deploy:
      replicas: 2

volumes:
  postgres-data:
  redis-data:
```

### Step 2: Set Up Cloudflare Tunnel

Cloudflare Tunnel creates a secure outbound connection from your server to Cloudflare's network.

#### Install cloudflared

```bash
# On Ubuntu/Debian
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb -o cloudflared.deb
sudo dpkg -i cloudflared.deb

# On macOS
brew install cloudflare/cloudflare/cloudflared
```

#### Create Tunnel

```bash
# Login to Cloudflare
cloudflared tunnel login

# Create a new tunnel
cloudflared tunnel create label-verify-hw

# This creates a credentials file at:
# ~/.cloudflared/<tunnel-id>.json
```

#### Configure Tunnel

Create `~/.cloudflared/config.yml`:

```yaml
tunnel: <your-tunnel-id>
credentials-file: /root/.cloudflared/<tunnel-id>.json

ingress:
  - hostname: api.yourdomain.com
    service: http://localhost:3000
  - service: http_status:404
```

#### Route DNS

```bash
# Create DNS route
cloudflared tunnel route dns label-verify-hw api.yourdomain.com
```

#### Run Tunnel

```bash
# As a service
cloudflared service install
sudo systemctl start cloudflared
sudo systemctl enable cloudflared

# Or in docker-compose (add to services):
cloudflared:
  image: cloudflare/cloudflared:latest
  command: tunnel --no-autoupdate run
  volumes:
    - ./cloudflared:/etc/cloudflared
  environment:
    TUNNEL_TOKEN: ${TUNNEL_TOKEN}
  restart: unless-stopped
```

### Step 3: Deploy to Cloud Platform

#### Deploy to DigitalOcean (Example)

```bash
# Create droplet
doctl compute droplet create label-verify-hw \
  --image ubuntu-22-04-x64 \
  --size s-2vcpu-4gb \
  --region nyc1

# SSH to droplet
ssh root@your-droplet-ip

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sh get-docker.sh

# Install Docker Compose
apt-get install docker-compose-plugin

# Clone repository
git clone <your-repo>
cd label-verify-hw

# Create .env file
nano .env.prod
# Add all environment variables

# Start services
docker compose up -d

# Install and configure cloudflared
# (follow steps from Step 2)
```

#### Deploy to AWS ECS (Alternative)

```bash
# Build and push to ECR
aws ecr create-repository --repository-name label-verify-hw-api
aws ecr create-repository --repository-name label-verify-hw-worker

# Build and tag
docker build -f Dockerfile.api -t label-verify-hw-api .
docker build -f Dockerfile.worker -t label-verify-hw-worker .

# Push to ECR
docker tag label-verify-hw-api:latest <account-id>.dkr.ecr.us-east-1.amazonaws.com/label-verify-hw-api:latest
docker push <account-id>.dkr.ecr.us-east-1.amazonaws.com/label-verify-hw-api:latest

# Create ECS task definitions and services
# Use RDS for PostgreSQL and ElastiCache for Redis
```

### Step 4: Configure Cloudflare Settings

#### Enable DDoS Protection

1. Go to Cloudflare Dashboard
2. Navigate to **Security** → **DDoS**
3. Enable **HTTP DDoS attack protection**
4. Set sensitivity to **High**

#### Configure WAF Rules

1. Go to **Security** → **WAF**
2. Create custom rules:

```
(http.request.uri.path contains "/api/v1/verify" and http.request.method eq "POST" and not http.request.headers["content-type"] contains "multipart/form-data")
```
Action: **Block**

#### Set Up Rate Limiting

1. Go to **Security** → **WAF** → **Rate limiting rules**
2. Create rule:
   - **Request**: 100 requests per minute
   - **Path**: `/api/v1/verify`
   - **Action**: Challenge or Block

#### Enable SSL/TLS

1. Go to **SSL/TLS** → **Overview**
2. Set mode to **Full (strict)**
3. Enable **Always Use HTTPS**
4. Enable **HTTP Strict Transport Security (HSTS)**

#### Configure Caching

1. Go to **Caching** → **Configuration**
2. **Cache Level**: Standard
3. Create Page Rule for `/api/*`:
   - **Cache Level**: Bypass (API responses shouldn't be cached)

### Step 5: Monitoring and Logging

#### Cloudflare Analytics

1. Go to **Analytics & Logs** → **Web Analytics**
2. Monitor:
   - Requests per second
   - Bandwidth usage
   - Threats blocked
   - Response times

#### Set Up Alerts

1. Go to **Notifications**
2. Create alerts for:
   - DDoS attacks
   - SSL certificate expiration
   - Origin health checks
   - Rate limit violations

---

## Environment Variables for Production

Create `.env.prod`:

```bash
# Database
DB_PASSWORD=<strong-random-password>
DATABASE_URL=postgresql://labelverify:${DB_PASSWORD}@postgres:5432/labelverify_prod

# Redis
REDIS_PASSWORD=<strong-random-password>
REDIS_URL=redis://:${REDIS_PASSWORD}@redis:6379

# Cloudflare
CF_ACCOUNT_ID=<your-account-id>
CF_API_TOKEN=<your-production-token>

# R2
R2_BUCKET=label-verify-prod
R2_ACCESS_KEY=<production-access-key>
R2_SECRET_KEY=<production-secret-key>
R2_ENDPOINT=https://<account-id>.r2.cloudflarestorage.com

# Encryption
ENCRYPTION_KEY=<base64-encoded-32-byte-key>

# Azure AD
AZURE_TENANT_ID=<your-tenant-id>
AZURE_CLIENT_ID=<your-client-id>

# Server
BIND_ADDR=0.0.0.0:3000
RUST_LOG=info

# Cloudflare Tunnel
TUNNEL_TOKEN=<your-tunnel-token>
```

---

## Security Checklist

- [ ] Generate strong random passwords for DB and Redis
- [ ] Rotate encryption keys from development
- [ ] Use scoped Cloudflare API tokens (production-only)
- [ ] Enable Cloudflare WAF and DDoS protection
- [ ] Set up rate limiting
- [ ] Enable HSTS with long max-age
- [ ] Configure firewall rules (only allow Cloudflare IPs)
- [ ] Enable audit logging
- [ ] Set up automated backups (PostgreSQL)
- [ ] Configure alerting for critical events
- [ ] Review and harden Docker containers
- [ ] Use secrets management (not .env files in production)

---

## Scaling Considerations

### Horizontal Scaling

Scale workers independently:

```yaml
# In docker-compose.yml
worker:
  # ...
  deploy:
    replicas: 5  # Run 5 worker instances
```

### Database Scaling

- Use PostgreSQL read replicas for job status queries
- Enable connection pooling (already configured)
- Consider managed PostgreSQL (AWS RDS, DigitalOcean Managed DB)

### Redis Scaling

- Use Redis Cluster for high availability
- Consider managed Redis (AWS ElastiCache, Redis Cloud)

### API Server Scaling

- Use load balancer (Cloudflare handles this)
- Scale horizontally with multiple API containers
- Monitor response times and scale accordingly

---

## Cost Estimation (Production)

### Cloudflare
- **Workers AI**: ~$10-50/month (depends on volume)
- **R2 Storage**: ~$5-20/month (depends on storage)
- **Tunnel**: Free
- **WAF/DDoS**: Included in Pro plan ($20/month)
- **Total Cloudflare**: ~$35-90/month

### Infrastructure (DigitalOcean example)
- **Droplet** (4GB RAM, 2 vCPU): $24/month
- **Managed PostgreSQL** (1GB): $15/month
- **Managed Redis** (1GB): $15/month
- **Backups**: ~$5/month
- **Total Infrastructure**: ~$59/month

### Alternative: AWS ECS
- **ECS Fargate** (2 vCPU, 4GB): ~$50/month
- **RDS PostgreSQL** (db.t3.small): ~$30/month
- **ElastiCache Redis** (cache.t3.micro): ~$15/month
- **Data Transfer**: ~$10/month
- **Total AWS**: ~$105/month

**Grand Total**: ~$94-195/month depending on platform

---

## Deployment Checklist

### Pre-Deployment
- [ ] Review code for production readiness
- [ ] Run security audit: `cargo audit`
- [ ] Run full test suite: `cargo test`
- [ ] Update documentation
- [ ] Generate production credentials
- [ ] Set up monitoring and alerting

### Deployment
- [ ] Build Docker images
- [ ] Push to container registry
- [ ] Provision infrastructure (DB, Redis)
- [ ] Deploy containers
- [ ] Run database migrations
- [ ] Configure Cloudflare Tunnel
- [ ] Set up DNS routing
- [ ] Enable Cloudflare protections

### Post-Deployment
- [ ] Verify health checks
- [ ] Test end-to-end flow
- [ ] Monitor logs for errors
- [ ] Load test with realistic traffic
- [ ] Set up automated backups
- [ ] Document rollback procedures

---

## Quick Start (Local Testing with Cloudflare Tunnel)

Test Cloudflare Tunnel locally before production:

```bash
# Start local services
docker compose up -d

# In another terminal, start tunnel
cloudflared tunnel --url http://localhost:3000

# Cloudflare provides a temporary URL (e.g., https://xyz.trycloudflare.com)
# Test your API through this URL
curl https://xyz.trycloudflare.com/health
```

---

## Next Steps

1. **Choose deployment platform** (DigitalOcean, AWS, GCP, Azure)
2. **Create Dockerfiles** (use templates above)
3. **Set up Cloudflare Tunnel**
4. **Deploy containers**
5. **Configure Cloudflare security**
6. **Monitor and optimize**

For questions or issues, refer to:
- [Cloudflare Tunnel Docs](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/)
- [Docker Documentation](https://docs.docker.com/)
- Project documentation in `docs/`

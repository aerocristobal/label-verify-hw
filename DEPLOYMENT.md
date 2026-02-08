# Deployment Guide - Quick Start

⚠️ **IMPORTANT: If you're seeing Cloudflare Pages build errors, [click here](#cloudflare-pages-build-error-fix)**

This guide covers deploying label-verify-hw to production using Docker and Cloudflare.

---

## 🚨 Cloudflare Pages Build Error Fix

**Are you seeing this error?**
```
❌ Error: .env.prod not found
Failed: error occurred while running deploy command
```

**This is a Rust API service with database dependencies - it CANNOT run on Cloudflare Pages.**

Cloudflare Pages is for static sites and edge functions. This service requires:
- Rust runtime ❌
- PostgreSQL database ❌
- Redis queue ❌
- Long-running processes ❌

### Quick Fix (Temporary)

**Option 1: Update Cloudflare Pages Settings**
1. Go to your Cloudflare Pages project → **Settings** → **Builds & deployments**
2. Change **Build command** to: `./build-cloudflare.sh`
3. Change **Build output directory** to: `dist`
4. Save and retry deployment

This creates a placeholder page explaining the issue.

**Option 2: Proper Deployment (Recommended)**

Deploy to a container platform instead:
- **Fly.io** (recommended for Rust): [See instructions](#flyio-deployment)
- **Railway**: [See instructions](#railway-deployment)
- **Render**: [See instructions](#render-deployment)
- **Docker Compose (VPS)**: [See instructions below](#-quick-deploy-docker--cloudflare-tunnel)

---

## 🚀 Quick Deploy (Docker + Cloudflare Tunnel)

### Prerequisites

- Docker and Docker Compose installed
- Cloudflare account with Workers AI and R2
- Domain name (optional, for Cloudflare Tunnel)

### Step 1: Configure Environment

```bash
# Copy production environment template
cp .env.prod.example .env.prod

# Edit with your production values
nano .env.prod
```

**Critical values to change**:
- `DB_PASSWORD` - Generate: `openssl rand -base64 32`
- `REDIS_PASSWORD` - Generate: `openssl rand -base64 32`
- `ENCRYPTION_KEY` - Generate: `openssl rand -base64 32` (DO NOT reuse dev key!)
- `CF_ACCOUNT_ID` - Your Cloudflare account ID
- `CF_API_TOKEN` - Production Workers AI token
- `R2_*` - Production R2 bucket credentials

### Step 2: Deploy Application

```bash
# Deploy all services
./deploy.sh deploy

# Check status
./deploy.sh status

# View logs
./deploy.sh logs
```

### Step 3: Test Deployment

```bash
# Run health checks
./deploy.sh test

# Manual test
curl http://localhost:3000/health
```

### Step 4: Set Up Cloudflare Tunnel (Optional)

```bash
# Install cloudflared
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o cloudflared
chmod +x cloudflared
sudo mv cloudflared /usr/local/bin/

# Login to Cloudflare
cloudflared tunnel login

# Create tunnel
cloudflared tunnel create label-verify-hw

# Save tunnel ID from output
TUNNEL_ID="<your-tunnel-id>"

# Create config
mkdir -p ~/.cloudflared
cat > ~/.cloudflared/config.yml <<EOF
tunnel: $TUNNEL_ID
credentials-file: /root/.cloudflared/$TUNNEL_ID.json

ingress:
  - hostname: api.yourdomain.com
    service: http://localhost:3000
  - service: http_status:404
EOF

# Route DNS
cloudflared tunnel route dns label-verify-hw api.yourdomain.com

# Run tunnel as service
cloudflared service install
sudo systemctl start cloudflared
sudo systemctl enable cloudflared
```

### Step 5: Scale Workers (Optional)

```bash
# Scale to 3 workers for higher throughput
./deploy.sh scale 3

# Check status
./deploy.sh status
```

## 📋 Deployment Commands

```bash
./deploy.sh deploy      # Deploy application
./deploy.sh logs        # View logs
./deploy.sh stop        # Stop services
./deploy.sh restart     # Restart services
./deploy.sh status      # Show status
./deploy.sh scale N     # Scale to N workers
./deploy.sh backup      # Create backup
./deploy.sh test        # Test deployment
./deploy.sh clean       # Remove all (WARNING: deletes data)
```

## 🔒 Security Checklist

Before deploying to production:

- [ ] Generated new random passwords (DB, Redis)
- [ ] Generated NEW encryption key (not reused from dev)
- [ ] Created production R2 bucket
- [ ] Generated scoped Cloudflare tokens (production only)
- [ ] Reviewed .env.prod for correctness
- [ ] Backed up encryption key securely
- [ ] Set up Cloudflare WAF and DDoS protection
- [ ] Enabled rate limiting
- [ ] Configured firewall rules
- [ ] Set up monitoring and alerts
- [ ] Tested backup and restore procedures

## 📊 Monitoring

### View Logs

```bash
# All services
docker compose --env-file .env.prod logs -f

# Specific service
docker compose --env-file .env.prod logs -f api
docker compose --env-file .env.prod logs -f worker
```

### Check Resource Usage

```bash
# Container stats
docker stats

# Disk usage
docker system df
```

### Cloudflare Analytics

1. Go to Cloudflare Dashboard
2. Navigate to Analytics & Logs
3. Monitor:
   - Requests per second
   - Workers AI usage
   - R2 storage and operations
   - Threats blocked

## 🔄 Updates and Maintenance

### Update Application

```bash
# Pull latest code
git pull origin main

# Rebuild and deploy
./deploy.sh deploy
```

### Backup Database

```bash
# Create backup
./deploy.sh backup

# Backups saved to: ./backups/YYYYMMDD_HHMMSS/
```

### Restore from Backup

```bash
# Stop services
./deploy.sh stop

# Restore database
cat ./backups/YYYYMMDD_HHMMSS/database.sql | \
  docker compose --env-file .env.prod exec -T postgres \
  psql -U labelverify labelverify_prod

# Start services
./deploy.sh deploy
```

## 🌐 Recommended Platforms (Alternatives to Cloudflare Pages)

### Fly.io Deployment

**Best for:** Rust applications, global deployment, managed databases

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Login
fly auth login

# Launch app (from project root)
fly launch --name label-verify-hw

# Create PostgreSQL
fly postgres create --name label-verify-db

# Create Redis
fly redis create --name label-verify-redis

# Set secrets
fly secrets set \
  DATABASE_URL="postgresql://..." \
  REDIS_URL="redis://..." \
  CF_ACCOUNT_ID="your-account-id" \
  CF_API_TOKEN="your-api-token" \
  R2_BUCKET="your-bucket" \
  R2_ACCESS_KEY="your-key" \
  R2_SECRET_KEY="your-secret" \
  R2_ENDPOINT="https://..." \
  ENCRYPTION_KEY="$(openssl rand -base64 32)"

# Deploy
fly deploy
```

**Resources:** [Fly.io Rust Guide](https://fly.io/docs/languages-and-frameworks/rust/)

---

### Railway Deployment

**Best for:** Simple setup, includes databases, GitHub integration

1. Go to [railway.app](https://railway.app)
2. Click **"New Project"** → **"Deploy from GitHub repo"**
3. Select `label-verify-hw` repository
4. Add **PostgreSQL** and **Redis** services
5. In **Variables** tab, add all environment variables
6. Deploy automatically on git push

**Resources:** [Railway Rust Guide](https://docs.railway.app/guides/rust)

---

### Render Deployment

**Best for:** Free tier, Docker support, managed databases

1. Go to [render.com](https://render.com)
2. Click **"New"** → **"Web Service"**
3. Connect GitHub repository
4. Choose **Docker** runtime
5. Set **Dockerfile path**: `Dockerfile.api`
6. Add PostgreSQL and Redis from Render dashboard
7. Configure environment variables
8. Deploy

**Resources:** [Render Docker Deployment](https://render.com/docs/docker)

---

## 🌐 Self-Hosted Deployment Platforms

### DigitalOcean

```bash
# Create droplet
doctl compute droplet create label-verify-hw \
  --image ubuntu-22-04-x64 \
  --size s-2vcpu-4gb \
  --region nyc1

# SSH and deploy
ssh root@<droplet-ip>
# Follow Quick Deploy steps above
```

### AWS EC2

```bash
# Launch EC2 instance (t3.medium or larger)
# Install Docker
curl -fsSL https://get.docker.com | sh

# Clone and deploy
git clone <your-repo>
cd label-verify-hw
# Follow Quick Deploy steps above
```

### Google Cloud Run (Alternative)

For fully managed deployment, see `docs/CLOUDFLARE_DEPLOYMENT.md` for Cloud Run instructions.

## 💰 Cost Estimation

**Monthly costs** (approximate):

| Service | Cost |
|---------|------|
| Cloudflare Workers AI | $10-50 |
| Cloudflare R2 | $5-20 |
| Cloudflare Pro Plan | $20 |
| VPS (4GB RAM) | $25-50 |
| Managed PostgreSQL | $15-30 |
| Managed Redis | $15-30 |
| **Total** | **$90-200/month** |

## 🆘 Troubleshooting

### Services won't start

```bash
# Check logs
./deploy.sh logs

# Common issues:
# - Missing .env.prod file
# - Invalid credentials
# - Port conflicts (3000, 5432, 6379)
```

### Database connection failed

```bash
# Check PostgreSQL health
docker compose --env-file .env.prod exec postgres pg_isready -U labelverify

# Check DATABASE_URL in .env.prod
```

### Worker not processing jobs

```bash
# Check worker logs
docker compose --env-file .env.prod logs worker

# Check Redis connection
docker compose --env-file .env.prod exec redis redis-cli ping

# Verify jobs in queue
docker compose --env-file .env.prod exec redis \
  redis-cli --no-auth-warning -a "$REDIS_PASSWORD" LLEN label_verify:jobs
```

### High memory usage

```bash
# Check container stats
docker stats

# Scale down workers
./deploy.sh scale 1

# Or increase server resources
```

## 📚 Further Reading

- Full deployment guide: `docs/CLOUDFLARE_DEPLOYMENT.md`
- Cloudflare setup: `docs/CLOUDFLARE_SETUP.md`
- CI/CD setup: `docs/CI_CD_SETUP.md`
- Quick reference: `docs/QUICK_REFERENCE.md`

## 🎯 Production Checklist

- [ ] Environment configured (.env.prod)
- [ ] Services deployed and healthy
- [ ] Cloudflare Tunnel configured (optional)
- [ ] SSL/TLS enabled
- [ ] Rate limiting configured
- [ ] Monitoring set up
- [ ] Backups scheduled
- [ ] Load tested
- [ ] Security audit completed
- [ ] Documentation reviewed

---

**Need help?** See `docs/CLOUDFLARE_DEPLOYMENT.md` for detailed instructions.

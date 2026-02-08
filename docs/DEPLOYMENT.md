# Deployment Guide - Quick Start

This guide covers deploying label-verify-hw to production using Docker and Cloudflare.

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

### Step 4: Set Up HTTPS via Cloudflare Tunnel

**Why Cloudflare Tunnel?**
- ✅ No port exposure needed (works behind NAT/firewalls)
- ✅ Automatic HTTPS with Cloudflare-managed certificates
- ✅ Built-in DDoS protection and WAF
- ✅ Zero configuration certificate management
- ✅ Integrates with existing Cloudflare Workers AI and R2

#### 4.1: Install cloudflared CLI

```bash
# macOS
brew install cloudflare/cloudflare/cloudflared

# Linux (Debian/Ubuntu)
wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb

# Verify installation
cloudflared --version
```

#### 4.2: Create and Configure Tunnel

```bash
# 1. Authenticate with Cloudflare
cloudflared tunnel login

# 2. Create tunnel
cloudflared tunnel create label-verify-hw

# 3. Create DNS record (replace with your domain)
cloudflared tunnel route dns label-verify-hw api.yourdomain.com

# 4. Get tunnel token (copy this output)
cloudflared tunnel token label-verify-hw
```

#### 4.3: Add Tunnel Token to Environment

Edit `.env.prod` and add:

```bash
# Cloudflare Tunnel
TUNNEL_TOKEN=eyJh...your_actual_token_here
DOMAIN_NAME=api.yourdomain.com
```

#### 4.4: Configure Cloudflare SSL Settings

1. Go to **Cloudflare Dashboard** → Your Domain → **SSL/TLS** → **Overview**
2. Set encryption mode: **Full**
3. Go to **SSL/TLS** → **Edge Certificates**
4. Enable **Always Use HTTPS**
5. Enable **HTTP Strict Transport Security (HSTS)** (recommended):
   - Max Age: 6 months (15768000 seconds)
   - Include subdomains: Yes
   - Preload: No
6. Enable **Automatic HTTPS Rewrites**

#### 4.5: Deploy with HTTPS

```bash
# Deploy all services (including cloudflared)
./deploy.sh deploy

# Verify tunnel is connected
docker logs labelverify-tunnel

# Expected output: "Registered tunnel connection"
```

#### 4.6: Test HTTPS Endpoint

```bash
# Test health check via HTTPS
curl https://api.yourdomain.com/health

# Expected: {"status":"ok","version":"0.1.0",...}

# Test HTTP redirect
curl -I http://api.yourdomain.com/health

# Expected: 301/302 redirect to HTTPS
```

#### 4.7: Optional - Remove Port Exposure

For production security, you can remove direct port access by editing `docker-compose.yml`:

```yaml
  api:
    # Comment out port exposure - traffic only via Cloudflare Tunnel
    # ports:
    #   - "3000:3000"
```

**Note:** Keep port exposed during testing, then remove for production deployment.

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
- [ ] **HTTPS Setup:**
  - [ ] Created Cloudflare Tunnel
  - [ ] Added TUNNEL_TOKEN to .env.prod
  - [ ] Set Cloudflare SSL mode to "Full"
  - [ ] Enabled "Always Use HTTPS"
  - [ ] Enabled HSTS
  - [ ] Tested HTTPS endpoint
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

## 🌐 Deployment Platforms

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

### Cloudflare Tunnel not connecting

```bash
# Check tunnel logs
docker logs labelverify-tunnel

# Verify TUNNEL_TOKEN is correct
grep TUNNEL_TOKEN .env.prod

# Check tunnel status in Cloudflare dashboard
# Dashboard → Traffic → Cloudflare Tunnel

# Common fixes:
# - Regenerate tunnel token: cloudflared tunnel token label-verify-hw
# - Update TUNNEL_TOKEN in .env.prod
# - Restart: ./deploy.sh restart
```

### HTTPS not working (502 Bad Gateway)

```bash
# Check API is healthy
docker compose --env-file .env.prod exec api curl http://localhost:3000/health

# Check tunnel is connected
docker logs labelverify-tunnel | grep -i "registered"

# Verify DNS record
dig api.yourdomain.com

# Expected: CNAME to <tunnel-id>.cfargotunnel.com

# Check Cloudflare SSL mode
# Dashboard → SSL/TLS → Should be "Full" or "Full (strict)"
```

## 📚 Further Reading

- Full deployment guide: `docs/CLOUDFLARE_DEPLOYMENT.md`
- Cloudflare setup: `docs/CLOUDFLARE_SETUP.md`
- CI/CD setup: `docs/CI_CD_SETUP.md`
- Quick reference: `docs/QUICK_REFERENCE.md`

## 🎯 Production Checklist

- [ ] Environment configured (.env.prod)
- [ ] Services deployed and healthy
- [ ] **HTTPS enabled via Cloudflare Tunnel:**
  - [ ] cloudflared tunnel created
  - [ ] DNS record configured
  - [ ] TUNNEL_TOKEN added to .env.prod
  - [ ] Cloudflare SSL settings configured
  - [ ] HTTPS endpoint tested
- [ ] Rate limiting configured
- [ ] Monitoring set up
- [ ] Backups scheduled
- [ ] Load tested
- [ ] Security audit completed
- [ ] Documentation reviewed

---

**Need help?** See `docs/CLOUDFLARE_DEPLOYMENT.md` for detailed instructions.

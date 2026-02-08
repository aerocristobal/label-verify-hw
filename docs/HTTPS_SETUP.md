# HTTPS Setup Guide - Cloudflare Tunnel

This guide covers enabling HTTPS for label-verify-hw using Cloudflare Tunnel.

## Why Cloudflare Tunnel?

Cloudflare Tunnel provides secure HTTPS access without exposing ports or managing certificates:

| Benefit | Description |
|---------|-------------|
| **Zero port exposure** | Works behind NAT/firewalls - no inbound ports needed |
| **Automatic HTTPS** | Cloudflare terminates TLS at their edge with auto-renewed certificates |
| **No certificate management** | No Certbot, Let's Encrypt, or manual certificate updates |
| **DDoS protection** | Built-in Cloudflare protection at the edge |
| **WAF integration** | Web Application Firewall for common attacks |
| **Rate limiting** | Edge-level rate limiting and bot protection |
| **Unified stack** | Integrates with existing Cloudflare Workers AI and R2 |

## Prerequisites

- Cloudflare account with a domain
- Domain DNS managed by Cloudflare (or ability to create CNAME record)
- Docker Compose environment already set up
- `cloudflared` CLI installed

## Step-by-Step Setup

### 1. Install cloudflared CLI

#### macOS
```bash
brew install cloudflare/cloudflare/cloudflared
```

#### Linux (Debian/Ubuntu)
```bash
wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64.deb
sudo dpkg -i cloudflared-linux-amd64.deb
```

#### Linux (RPM-based)
```bash
wget -q https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-x86_64.rpm
sudo rpm -i cloudflared-linux-x86_64.rpm
```

#### Verify Installation
```bash
cloudflared --version
# Expected: cloudflared version 2024.x.x (or later)
```

---

### 2. Authenticate with Cloudflare

```bash
cloudflared tunnel login
```

This opens a browser window. Steps:
1. Select your Cloudflare account
2. Choose the domain you want to use
3. Authorize cloudflared

A certificate file is saved to `~/.cloudflared/cert.pem`.

---

### 3. Create the Tunnel

```bash
cloudflared tunnel create label-verify-hw
```

**Output:**
```
Tunnel credentials written to /Users/you/.cloudflared/<TUNNEL_ID>.json
Created tunnel label-verify-hw with id <TUNNEL_ID>
```

**Important:** Keep the tunnel ID - you'll need it for DNS routing.

---

### 4. Route DNS to Tunnel

Replace `api.yourdomain.com` with your desired subdomain:

```bash
cloudflared tunnel route dns label-verify-hw api.yourdomain.com
```

**Output:**
```
Created CNAME record: api.yourdomain.com -> <TUNNEL_ID>.cfargotunnel.com
```

**Verify DNS:**
```bash
dig api.yourdomain.com

# Expected: CNAME record pointing to *.cfargotunnel.com
```

---

### 5. Get Tunnel Token

```bash
cloudflared tunnel token label-verify-hw
```

**Output:**
```
eyJhIjoiYzk4ZTM4NzE3ZjAwMmY4MjNmNjA0ZTQzN2Y4YmY0OGEiLCJ0IjoiZjVkMWQ3NzAtMzRlZC00NzE...
```

**Copy this entire token** (starts with `eyJ`). You'll add it to `.env.prod`.

---

### 6. Configure Environment

Edit `.env.prod` and add:

```bash
# ============================================================================
# CLOUDFLARE TUNNEL (HTTPS)
# ============================================================================

TUNNEL_TOKEN=eyJhIjoiYzk4ZTM4NzE3ZjAwMmY4MjNmNjA0ZTQzN2Y4YmY0OGEiLCJ0IjoiZjVkMWQ3NzAtMzRlZC00NzE...
DOMAIN_NAME=api.yourdomain.com
```

**Note:** The tunnel configuration is already enabled in `docker-compose.yml`. The `cloudflared` service will start automatically when you deploy.

---

### 7. Configure Cloudflare SSL Settings

#### 7.1: Set SSL/TLS Encryption Mode

1. Go to **Cloudflare Dashboard** → Your Domain
2. Navigate to **SSL/TLS** → **Overview**
3. Set encryption mode: **Full**

**Why Full?**
- Encrypts traffic between Cloudflare and your origin
- Tunnel itself provides encryption (no origin certificate needed)
- For stricter validation, use **Full (strict)** with a valid origin cert

#### 7.2: Enable Always Use HTTPS

1. Go to **SSL/TLS** → **Edge Certificates**
2. Enable **Always Use HTTPS**
3. This redirects all HTTP requests to HTTPS automatically

#### 7.3: Enable HSTS (Recommended)

1. In **SSL/TLS** → **Edge Certificates**
2. Enable **HTTP Strict Transport Security (HSTS)**
3. Settings:
   - **Max Age Header**: 6 months (15768000 seconds)
   - **Include subdomains**: Yes (if you want subdomains covered)
   - **Preload**: No (unless you plan to submit to browser preload list)

**Warning:** HSTS is sticky in browsers. Once enabled, browsers will refuse to connect over HTTP even if you disable it. Test thoroughly before enabling with long max-age.

#### 7.4: Enable Automatic HTTPS Rewrites

1. In **SSL/TLS** → **Edge Certificates**
2. Enable **Automatic HTTPS Rewrites**
3. This rewrites insecure resource requests to HTTPS

---

### 8. Deploy Application

```bash
# Deploy all services (including cloudflared tunnel)
docker compose --env-file .env.prod up -d

# Check all services are running
docker compose --env-file .env.prod ps

# Expected output should show:
# - labelverify-postgres (healthy)
# - labelverify-redis (healthy)
# - labelverify-api (healthy)
# - labelverify-worker (running)
# - labelverify-tunnel (running)
```

---

### 9. Verify Tunnel Connection

```bash
# Check tunnel logs
docker logs labelverify-tunnel

# Expected output (should see these lines):
# "Registered tunnel connection"
# "Connection <uuid> registered"
```

**Tunnel Status in Dashboard:**
1. Go to **Cloudflare Dashboard** → **Traffic** → **Cloudflare Tunnel**
2. Find "label-verify-hw" tunnel
3. Status should be **HEALTHY** with active connections

---

### 10. Test HTTPS Endpoint

#### 10.1: Test Health Check
```bash
curl https://api.yourdomain.com/health

# Expected output:
# {"status":"ok","version":"0.1.0","database":"connected","redis":"connected"}
```

#### 10.2: Test HTTP Redirect
```bash
curl -I http://api.yourdomain.com/health

# Expected output:
# HTTP/1.1 301 Moved Permanently
# Location: https://api.yourdomain.com/health
```

#### 10.3: Test Web UI
```bash
# Open in browser
open https://api.yourdomain.com

# Expected: Label verification form loads with valid HTTPS (lock icon)
```

#### 10.4: Test SSL Certificate
```bash
echo | openssl s_client -connect api.yourdomain.com:443 -servername api.yourdomain.com 2>/dev/null | openssl x509 -noout -issuer -subject -dates

# Expected issuer: Cloudflare
```

#### 10.5: Test API Upload
```bash
# Upload a test label image
curl -X POST https://api.yourdomain.com/api/v1/verify \
  -F "image=@tests/test_label1.jpeg"

# Expected:
# {"job_id":"<uuid>","status":"pending","created_at":"2026-02-08T..."}
```

---

## Optional: Remove Port Exposure

For production security, you can remove direct HTTP access on port 3000.

Edit `docker-compose.yml`:

```yaml
  api:
    build:
      context: .
      dockerfile: Dockerfile.api
    container_name: labelverify-api
    # Port 3000 not exposed externally - only accessible via Cloudflare Tunnel
    # Uncomment for local testing without tunnel:
    # ports:
    #   - "3000:3000"
    environment:
      # ... rest of config
```

**Then restart:**
```bash
docker compose --env-file .env.prod up -d --force-recreate api
```

**After this change:**
- ✅ HTTPS via tunnel: `https://api.yourdomain.com` works
- ❌ Direct HTTP access: `http://your-server-ip:3000` is blocked

---

## Advanced Configuration (Optional)

### Custom Ingress Rules

For advanced routing (multiple services, path-based routing), create a config file:

**Create** `cloudflared/config.yml`:
```yaml
tunnel: <TUNNEL_ID>
credentials-file: /etc/cloudflared/creds.json

ingress:
  # Route API traffic
  - hostname: api.yourdomain.com
    service: http://api:3000
    originRequest:
      noTLSVerify: false
      connectTimeout: 30s
      keepAliveTimeout: 90s

  # Catch-all rule (required)
  - service: http_status:404
```

**Update** `docker-compose.yml`:
```yaml
  cloudflared:
    image: cloudflare/cloudflared:latest
    container_name: labelverify-tunnel
    command: tunnel --config /etc/cloudflared/config.yml run
    environment:
      TUNNEL_TOKEN: ${TUNNEL_TOKEN}
    volumes:
      - ./cloudflared/config.yml:/etc/cloudflared/config.yml:ro
    # ... rest of config
```

---

## Troubleshooting

### Issue: Tunnel not connecting

**Symptoms:**
- `docker logs labelverify-tunnel` shows connection errors
- Dashboard shows tunnel as INACTIVE

**Solutions:**

1. **Verify TUNNEL_TOKEN is correct:**
   ```bash
   grep TUNNEL_TOKEN .env.prod
   # Should show full token starting with eyJ...
   ```

2. **Regenerate token:**
   ```bash
   cloudflared tunnel token label-verify-hw
   # Copy new token to .env.prod
   ```

3. **Check tunnel exists:**
   ```bash
   cloudflared tunnel list
   # Should show label-verify-hw tunnel
   ```

4. **Recreate tunnel if needed:**
   ```bash
   cloudflared tunnel delete label-verify-hw
   cloudflared tunnel create label-verify-hw
   cloudflared tunnel route dns label-verify-hw api.yourdomain.com
   cloudflared tunnel token label-verify-hw
   # Update TUNNEL_TOKEN in .env.prod
   ```

5. **Restart services:**
   ```bash
   docker compose --env-file .env.prod restart cloudflared
   docker logs labelverify-tunnel -f
   ```

---

### Issue: 502 Bad Gateway

**Symptoms:**
- `https://api.yourdomain.com` returns 502 error
- Browser shows "Bad Gateway"

**Solutions:**

1. **Check API is healthy:**
   ```bash
   docker compose --env-file .env.prod exec api curl http://localhost:3000/health
   # Should return {"status":"ok",...}
   ```

2. **Check API container is running:**
   ```bash
   docker compose --env-file .env.prod ps api
   # State should be "Up" with (healthy)
   ```

3. **Check API logs for errors:**
   ```bash
   docker logs labelverify-api --tail 100
   ```

4. **Verify tunnel routing:**
   - Go to Cloudflare Dashboard → Traffic → Cloudflare Tunnel
   - Click "label-verify-hw" → Configure
   - Verify hostname routes to `http://api:3000`

5. **Check network connectivity:**
   ```bash
   docker compose --env-file .env.prod exec cloudflared ping api
   # Should successfully ping api container
   ```

---

### Issue: SSL certificate invalid

**Symptoms:**
- Browser shows certificate warning
- Certificate not issued by Cloudflare

**Solutions:**

1. **Wait for DNS propagation:**
   ```bash
   dig api.yourdomain.com
   # Should show CNAME to *.cfargotunnel.com
   # Can take up to 24h (usually <5min)
   ```

2. **Verify Cloudflare SSL mode:**
   - Dashboard → SSL/TLS → Overview
   - Should be "Full" or "Full (strict)"

3. **Clear browser cache:**
   - Chrome: Ctrl+Shift+Delete → Cached images and files
   - Safari: Cmd+Option+E

4. **Purge Cloudflare cache:**
   - Dashboard → Caching → Configuration
   - Click "Purge Everything"

---

### Issue: HTTP not redirecting to HTTPS

**Symptoms:**
- `curl http://api.yourdomain.com` doesn't redirect
- HTTP and HTTPS both work

**Solutions:**

1. **Enable "Always Use HTTPS":**
   - Dashboard → SSL/TLS → Edge Certificates
   - Enable "Always Use HTTPS"

2. **Wait for edge cache update:**
   - Takes 2-3 minutes for settings to propagate
   - Test with `curl -I http://api.yourdomain.com`

3. **Purge cache:**
   - Dashboard → Caching → Configuration
   - Click "Purge Everything"

---

### Issue: Tunnel disconnects randomly

**Symptoms:**
- Tunnel status flaps between HEALTHY and INACTIVE
- Intermittent 502 errors

**Solutions:**

1. **Check API container stability:**
   ```bash
   docker compose --env-file .env.prod ps
   # All containers should show "Up" state
   ```

2. **Increase tunnel timeout:**

   Create `cloudflared/config.yml`:
   ```yaml
   tunnel: <TUNNEL_ID>
   credentials-file: /etc/cloudflared/creds.json

   ingress:
     - hostname: api.yourdomain.com
       service: http://api:3000
       originRequest:
         keepAliveTimeout: 120s
         noHappyEyeballs: true
     - service: http_status:404
   ```

3. **Check resource limits:**
   ```bash
   docker stats
   # Ensure containers aren't hitting memory/CPU limits
   ```

---

## Security Best Practices

### 1. Firewall Configuration

With Cloudflare Tunnel, you can completely block public access to port 3000:

```bash
# Block port 3000 (ufw)
sudo ufw deny 3000

# Block port 3000 (iptables)
sudo iptables -A INPUT -p tcp --dport 3000 -j DROP
```

**All traffic routes through Cloudflare Tunnel**, so direct port access is unnecessary.

---

### 2. Enable Cloudflare WAF

1. Go to **Security** → **WAF**
2. Enable **OWASP Core Ruleset**
3. Enable **Cloudflare Managed Ruleset**
4. Consider enabling:
   - **Cloudflare Bot Management** (Pro plan+)
   - **Rate Limiting** (under Security → WAF → Rate Limiting Rules)

---

### 3. Configure Rate Limiting

1. Go to **Security** → **WAF** → **Rate Limiting Rules**
2. Create rule:
   - **Rule name**: API Upload Limit
   - **When incoming requests match**: `http.request.uri.path contains "/api/v1/verify"`
   - **Request rate**: 10 requests per minute
   - **Action**: Block

---

### 4. IP Allowlisting (Optional)

For internal-only deployments, restrict access by IP:

1. Go to **Security** → **WAF** → **Firewall rules**
2. Create rule:
   - **Rule name**: Allow internal IPs only
   - **Expression**: `ip.src in {1.2.3.4 5.6.7.8}`
   - **Action**: Allow
3. Create second rule:
   - **Expression**: `true`
   - **Action**: Block

---

### 5. Monitor Access Logs

Enable **Logpush** (Enterprise) or use **Log Explorer**:

1. Go to **Analytics & Logs** → **Logs** → **Logpush**
2. Configure destination (S3, R2, etc.)
3. Monitor for:
   - Unusual traffic patterns
   - Failed authentication attempts
   - Geographic anomalies

---

## Migration from HTTP to HTTPS

If you're already running on HTTP and want to migrate:

### Step 1: Enable tunnel with HTTP still accessible
```bash
# Keep port 3000 exposed
# Add TUNNEL_TOKEN to .env.prod
docker compose --env-file .env.prod up -d
```

### Step 2: Test HTTPS works
```bash
curl https://api.yourdomain.com/health
```

### Step 3: Enable "Always Use HTTPS" in Cloudflare
- This redirects HTTP → HTTPS

### Step 4: Monitor logs for 24-48 hours
```bash
docker logs labelverify-tunnel -f
docker logs labelverify-api -f
```

### Step 5: Remove port exposure (optional)
```yaml
# docker-compose.yml
api:
  # Comment out:
  # ports:
  #   - "3000:3000"
```

### Step 6: Update monitoring and alerts
- Update healthchecks to use HTTPS URLs
- Update any hardcoded HTTP URLs in code/docs

---

## Cost Considerations

| Item | Cost |
|------|------|
| Cloudflare Tunnel | **Free** |
| SSL/TLS certificate | **Free** (auto-managed by Cloudflare) |
| DDoS protection | **Free** (on all plans) |
| WAF (managed rulesets) | **Free** (on Pro plan $20/mo) |
| Bot Management | Requires Pro or higher ($20+/mo) |
| Rate Limiting | Free (limited rules) |

**Total HTTPS cost: $0** (using free Cloudflare plan)

---

## Rollback Strategy

If you need to revert to HTTP-only:

### Quick Rollback (keep tunnel for later)
```bash
# Stop cloudflared
docker compose --env-file .env.prod stop cloudflared

# Re-expose API port if removed
# Edit docker-compose.yml: uncomment ports: ["3000:3000"]
docker compose --env-file .env.prod up -d api

# Access via HTTP
curl http://your-server-ip:3000/health
```

### Full Rollback (remove tunnel)
```bash
# Stop and remove cloudflared
docker compose --env-file .env.prod rm -f cloudflared

# Delete tunnel
cloudflared tunnel delete label-verify-hw

# Remove DNS record
# Go to Cloudflare Dashboard → DNS → Delete CNAME for api.yourdomain.com

# Remove from .env.prod
# Delete TUNNEL_TOKEN and DOMAIN_NAME lines
```

---

## Monitoring and Alerts

### Tunnel Health Check

Create a simple monitoring script:

**`scripts/check_tunnel.sh`:**
```bash
#!/bin/bash
set -e

DOMAIN="api.yourdomain.com"
EXPECTED_STATUS=200

# Check HTTPS endpoint
STATUS=$(curl -s -o /dev/null -w "%{http_code}" https://$DOMAIN/health)

if [ "$STATUS" -eq "$EXPECTED_STATUS" ]; then
  echo "✅ Tunnel healthy: HTTPS $DOMAIN (HTTP $STATUS)"
  exit 0
else
  echo "❌ Tunnel unhealthy: HTTPS $DOMAIN (HTTP $STATUS)"
  exit 1
fi
```

**Run via cron:**
```bash
# Check every 5 minutes
*/5 * * * * /path/to/scripts/check_tunnel.sh || echo "Tunnel down" | mail -s "Alert: Cloudflare Tunnel Down" admin@example.com
```

---

### Cloudflare Analytics

Monitor in Cloudflare Dashboard:

1. **Analytics & Logs** → **Traffic**:
   - Requests per second
   - Bandwidth usage
   - Status codes (watch for 502s)
   - Geographic distribution

2. **Analytics & Logs** → **Performance**:
   - Origin response time
   - Edge response time
   - Time to first byte (TTFB)

3. **Security** → **Events**:
   - Threats blocked
   - Challenge rate
   - WAF events

---

## Next Steps

After HTTPS is working:

1. ✅ Enable **Cloudflare WAF** (see [Security Best Practices](#security-best-practices))
2. ✅ Configure **Rate Limiting** for API endpoints
3. ✅ Set up **monitoring and alerts** for tunnel health
4. ✅ Update **API documentation** with HTTPS URLs
5. ✅ Update **client applications** to use HTTPS
6. ✅ Consider **Cloudflare Access** for authentication layer
7. ✅ Review **compliance requirements** (HIPAA, SOC2, etc.)

---

## Related Documentation

- [Deployment Guide](DEPLOYMENT.md) - Full production deployment
- [Cloudflare Setup](CLOUDFLARE_SETUP.md) - Workers AI and R2 configuration
- [Security Guide](../SECURITY.md) - Security best practices
- [Cloudflare Tunnel Docs](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/) - Official documentation

---

## Support

**Issues with this setup?**

1. Check logs: `docker logs labelverify-tunnel`
2. Review [Troubleshooting](#troubleshooting) section
3. Verify Cloudflare dashboard → Tunnel status
4. Test with: `curl -v https://api.yourdomain.com/health`

**Still stuck?**

- Open GitHub issue: https://github.com/aerocristobal/label-verify-hw/issues
- Include:
  - Docker logs (`docker logs labelverify-tunnel`)
  - Tunnel status from Cloudflare Dashboard
  - Output of `dig api.yourdomain.com`
  - Any error messages

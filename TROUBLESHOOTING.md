# Deployment Troubleshooting Guide

Common issues and solutions for deploying label-verify-hw.

---

## Docker Build Errors

### Error: "Cargo.lock not found"

**Symptom**:
```
ERROR: failed to calculate checksum: "/Cargo.lock": not found
```

**Cause**: Cargo.lock was in .gitignore or .dockerignore

**Solution**: ✅ **FIXED**
- Cargo.lock is now committed to the repository
- Cargo.lock is required for reproducible Docker builds
- This is the correct approach for Rust applications (vs libraries)

---

### Error: Missing Environment Variables

**Symptom**:
```
level=warning msg="The \"DB_PASSWORD\" variable is not set. Defaulting to a blank string."
```

**Cause**: No .env.prod file configured

**Solution**:
```bash
# 1. Copy template
cp .env.prod.example .env.prod

# 2. Edit with your values
nano .env.prod

# Required minimum values:
# - DB_PASSWORD (generate: openssl rand -base64 32)
# - REDIS_PASSWORD (generate: openssl rand -base64 32)
# - CF_ACCOUNT_ID
# - CF_API_TOKEN
# - R2_BUCKET
# - R2_ACCESS_KEY
# - R2_SECRET_KEY
# - R2_ENDPOINT
# - ENCRYPTION_KEY (generate: openssl rand -base64 32)
```

---

### Warning: Docker Compose Version

**Symptom**:
```
level=warning msg="the attribute `version` is obsolete"
```

**Cause**: Docker Compose v2+ doesn't need version field

**Solution**: This is just a warning, safe to ignore. Or remove version line from docker-compose.yml.

---

## Runtime Errors

### Error: Cannot Connect to PostgreSQL

**Symptom**:
```
Error: Failed to connect to database
```

**Solutions**:

1. **Check PostgreSQL is running**:
```bash
docker compose --env-file .env.prod ps postgres
```

2. **Check DATABASE_URL**:
```bash
# Should be:
postgresql://labelverify:${DB_PASSWORD}@postgres:5432/labelverify_prod
```

3. **Test connection manually**:
```bash
docker compose --env-file .env.prod exec postgres \
  psql -U labelverify -d labelverify_prod
```

4. **Check logs**:
```bash
docker compose --env-file .env.prod logs postgres
```

---

### Error: Cannot Connect to Redis

**Symptom**:
```
Error: Redis error: Connection refused
```

**Solutions**:

1. **Check Redis is running**:
```bash
docker compose --env-file .env.prod ps redis
```

2. **Check REDIS_URL**:
```bash
# Should be:
redis://:${REDIS_PASSWORD}@redis:6379
```

3. **Test connection**:
```bash
docker compose --env-file .env.prod exec redis \
  redis-cli --no-auth-warning -a "${REDIS_PASSWORD}" ping
```

---

### Error: Cloudflare API Errors

**Symptom**:
```
Error: HTTP 401 - Invalid API token
```

**Solutions**:

1. **Verify CF_API_TOKEN**:
- Check token hasn't expired
- Verify token has Workers AI → Read permission
- Ensure token is for correct account

2. **Test Workers AI**:
```bash
curl -X POST \
  https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/ai/run/@cf/llava-hf/llava-1.5-7b-hf \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -d '{"prompt":"test","image":"base64...","max_tokens":10}'
```

3. **Check R2 credentials**:
```bash
# Test with AWS CLI (S3-compatible)
aws s3 ls --endpoint-url $R2_ENDPOINT \
  --profile r2
```

---

### Error: Encryption/Decryption Failed

**Symptom**:
```
Error: Decryption failed
```

**Causes & Solutions**:

1. **Wrong encryption key**:
- Ensure ENCRYPTION_KEY is base64-encoded 32 bytes
- Generate correctly: `openssl rand -base64 32`

2. **Key changed**:
- If you change ENCRYPTION_KEY, old encrypted data cannot be decrypted
- **Never lose or change this key in production**

3. **Corrupted data**:
- Check R2 storage integrity
- Verify image upload completed successfully

---

### Worker Not Processing Jobs

**Symptom**: Jobs stuck in "pending" status

**Solutions**:

1. **Check worker is running**:
```bash
docker compose --env-file .env.prod ps worker
```

2. **Check worker logs**:
```bash
docker compose --env-file .env.prod logs worker
```

3. **Verify Redis queue**:
```bash
docker compose --env-file .env.prod exec redis \
  redis-cli --no-auth-warning -a "${REDIS_PASSWORD}" LLEN label_verify:jobs
```

4. **Restart worker**:
```bash
docker compose --env-file .env.prod restart worker
```

5. **Scale workers**:
```bash
./deploy.sh scale 3  # Run 3 workers
```

---

## Performance Issues

### Slow Upload Response

**Symptoms**: Upload takes >10 seconds

**Solutions**:

1. **Check image size**:
- Maximum: 10MB
- Optimize images before upload

2. **Check R2 latency**:
```bash
time curl -X PUT $R2_ENDPOINT/test-object \
  --data-binary @test-file.jpg
```

3. **Check encryption overhead**:
- Should be <100ms for typical images
- Monitor CPU usage

4. **Network issues**:
- Check connection to Cloudflare R2
- Verify no firewall blocking

---

### Worker Processing Slow

**Symptoms**: OCR takes >30 seconds

**Solutions**:

1. **Check Workers AI latency**:
- Normal: 3-7 seconds
- If >10 seconds, check Cloudflare status

2. **Check database queries**:
```bash
# Enable query logging
RUST_LOG=debug ./deploy.sh restart
```

3. **Scale workers**:
```bash
./deploy.sh scale 5
```

---

## Container Issues

### Container Keeps Restarting

**Solutions**:

1. **Check logs**:
```bash
docker compose --env-file .env.prod logs api
docker compose --env-file .env.prod logs worker
```

2. **Common causes**:
- Missing environment variables
- Cannot connect to database/redis
- Invalid Cloudflare credentials
- Port conflict (3000 already in use)

3. **Check health**:
```bash
docker compose --env-file .env.prod ps
```

---

### Out of Memory

**Symptoms**:
```
Container killed (OOMKilled)
```

**Solutions**:

1. **Check container stats**:
```bash
docker stats
```

2. **Increase Docker memory**:
- Docker Desktop: Settings → Resources → Memory
- Linux: Update Docker daemon config

3. **Reduce workers**:
```bash
./deploy.sh scale 1
```

4. **Optimize images**:
- Use smaller base images
- Multi-stage builds (already implemented)

---

### Disk Space Issues

**Symptoms**:
```
Error: No space left on device
```

**Solutions**:

1. **Check disk usage**:
```bash
docker system df
```

2. **Clean up**:
```bash
# Remove unused images
docker image prune -a

# Remove unused volumes
docker volume prune

# Complete cleanup (WARNING: removes all stopped containers)
docker system prune -a
```

3. **Check logs size**:
```bash
# Limit log size in docker-compose.yml
logging:
  driver: "json-file"
  options:
    max-size: "10m"
    max-file: "3"
```

---

## Development vs Production

### Works Locally, Fails in Production

**Check**:

1. **Environment variables**:
```bash
# Compare .env vs .env.prod
diff .env .env.prod
```

2. **Network accessibility**:
- Can production access Cloudflare APIs?
- Firewall rules blocking outbound?

3. **Resource limits**:
- Production has less RAM/CPU?
- Need to scale down workers

4. **Database migrations**:
```bash
# Run migrations manually
docker compose --env-file .env.prod exec api \
  sqlx migrate run
```

---

## Quick Diagnostics

### Full System Check

```bash
#!/bin/bash

echo "=== System Check ==="

# Check Docker
echo "Docker version:"
docker --version

# Check services
echo -e "\nService status:"
docker compose --env-file .env.prod ps

# Check health
echo -e "\nHealth checks:"
curl -f http://localhost:3000/health && echo "✅ API healthy" || echo "❌ API unhealthy"

# Check database
docker compose --env-file .env.prod exec postgres \
  pg_isready -U labelverify && echo "✅ Database ready" || echo "❌ Database not ready"

# Check Redis
docker compose --env-file .env.prod exec redis \
  redis-cli --no-auth-warning -a "${REDIS_PASSWORD}" ping && echo "✅ Redis ready" || echo "❌ Redis not ready"

# Check logs for errors
echo -e "\nRecent errors:"
docker compose --env-file .env.prod logs --tail=50 | grep -i error

echo -e "\n=== Check Complete ==="
```

Save as `check-health.sh` and run:
```bash
chmod +x check-health.sh
./check-health.sh
```

---

## Getting Help

### Before Asking for Help

Collect this information:

1. **Error message** (full stack trace)
2. **Logs**:
```bash
docker compose --env-file .env.prod logs > logs.txt
```
3. **Configuration** (sanitized, no secrets)
4. **Environment**: OS, Docker version
5. **What you've tried** already

### Resources

- **Project README**: `README.md`
- **Deployment Guide**: `DEPLOYMENT.md`
- **Cloudflare Setup**: `docs/CLOUDFLARE_SETUP.md`
- **Docker Docs**: https://docs.docker.com
- **Cloudflare Docs**: https://developers.cloudflare.com

---

## Preventive Measures

### Before Production

- [ ] Test full deployment on staging
- [ ] Run load tests
- [ ] Verify backups work
- [ ] Test disaster recovery
- [ ] Document all credentials securely
- [ ] Set up monitoring and alerts
- [ ] Test failover scenarios
- [ ] Review security settings

### Regular Maintenance

- [ ] Weekly: Check logs for errors
- [ ] Weekly: Monitor resource usage
- [ ] Monthly: Test backups
- [ ] Monthly: Review security
- [ ] Quarterly: Rotate credentials
- [ ] Quarterly: Update dependencies

---

**Remember**: When in doubt, check the logs first!
```bash
./deploy.sh logs
```

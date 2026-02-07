# Cloudflare Quick Reference

Quick reference for working with Cloudflare credentials in label-verify-hw.

---

## Environment Variables

```bash
# Cloudflare Workers AI
CF_ACCOUNT_ID=your_account_id_here
CF_API_TOKEN=your_workers_ai_token_here

# Cloudflare R2 Storage
R2_BUCKET=label-verify-{env}
R2_ACCESS_KEY=your_r2_access_key_here
R2_SECRET_KEY=your_r2_secret_key_here
R2_ENDPOINT=https://your_account_id.r2.cloudflarestorage.com
```

---

## Bucket Names by Environment

| Environment | Bucket Name |
|-------------|-------------|
| Development | `label-verify-dev` |
| Testing (CI/CD) | `label-verify-test` |
| Staging | `label-verify-staging` |
| Production | `label-verify-prod` |

---

## Quick Setup

### 1. Get Account ID
Dashboard → Any domain → API section (right sidebar)

### 2. Create Workers AI Token
[API Tokens](https://dash.cloudflare.com/profile/api-tokens) → Create Token → Custom Token
- Permission: Account → Workers AI → Read

### 3. Create R2 Bucket
[R2 Dashboard](https://dash.cloudflare.com/r2) → Create bucket

### 4. Generate R2 Token
R2 Dashboard → Manage R2 API Tokens → Create API token
- Permissions: Object Read & Write
- Scope: Specific bucket only

### 5. Configure .env
```bash
cp .env.example .env
# Edit .env with your credentials
```

---

## Test Connectivity

### Test Workers AI
```bash
curl -X POST \
  https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/ai/run/@cf/llava-hf/llava-1.5-7b-hf \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -d '{"prompt":"test","image":"<base64>","max_tokens":10}'
```

### Test R2 Storage
```bash
cargo run --example test_r2
```

### Test Workers AI (Rust)
```bash
cargo run --example test_workers_ai
```

---

## Common Commands

```bash
# Build project
cargo build

# Run server (loads .env automatically)
cargo run

# Run tests
cargo test

# Run specific example
cargo run --example test_r2
cargo run --example test_workers_ai

# Check for security issues
cargo audit

# Lint code
cargo clippy
```

---

## Troubleshooting

### "Invalid API token"
- Check token hasn't expired
- Verify token has Workers AI → Read permission
- Ensure using `Bearer` prefix: `Authorization: Bearer $TOKEN`

### "Access denied" on R2
- Verify token is scoped to correct bucket
- Check bucket name matches `R2_BUCKET` env var
- Confirm token has Read+Write permissions

### "Account ID not found"
- Double-check account ID from dashboard
- Remove any whitespace from env var
- Verify account has Workers AI enabled

---

## Cost Estimates

| Environment | Workers AI | R2 Storage | Total/Month |
|-------------|-----------|------------|-------------|
| Development | ~$1-5 | ~$1 | ~$5-10 |
| Testing | ~$1-5 | <$1 | ~$1-5 |
| Staging | ~$10-50 | ~$5-10 | ~$15-60 |

---

## Security Checklist

- [ ] `.env` file is in `.gitignore`
- [ ] Using scoped API tokens (not global keys)
- [ ] Tokens have expiration dates set
- [ ] Separate credentials per environment
- [ ] Secret scanning enabled
- [ ] Credentials stored securely (not in code)

---

## Useful Links

- **Full Setup Guide**: [CLOUDFLARE_SETUP.md](./CLOUDFLARE_SETUP.md)
- **CI/CD Configuration**: [CI_CD_SETUP.md](./CI_CD_SETUP.md)
- **Cloudflare Dashboard**: https://dash.cloudflare.com
- **API Tokens**: https://dash.cloudflare.com/profile/api-tokens
- **R2 Dashboard**: https://dash.cloudflare.com/r2
- **Workers AI Docs**: https://developers.cloudflare.com/workers-ai/
- **R2 Docs**: https://developers.cloudflare.com/r2/

---

## Emergency Procedures

### Leaked Credentials
1. **Immediately revoke** the compromised token in Cloudflare dashboard
2. Generate new token with same permissions
3. Update `.env` file (or CI/CD secrets)
4. Audit access logs for unauthorized usage
5. Rotate other related credentials as precaution

### Lost ENCRYPTION_KEY
⚠️ **Critical**: Cannot decrypt stored images without this key!
1. Check backup secret storage system
2. If truly lost, all encrypted images are unrecoverable
3. Generate new key: `openssl rand -base64 32`
4. Update `.env` and redeploy
5. Note: Old images will be permanently inaccessible

### High Costs
1. Check Cloudflare Analytics for usage spikes
2. Review application logs for retry loops
3. Verify lifecycle policies are working
4. Check for leaked credentials (unauthorized usage)
5. Implement rate limiting if not already enabled

---

## Support

- **Project Issues**: Create issue in GitHub repo
- **Cloudflare Support**: https://support.cloudflare.com/
- **Community**: https://community.cloudflare.com/

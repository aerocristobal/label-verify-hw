# Cloudflare API Keys Setup Guide

## Overview

The label-verify-hw application requires Cloudflare credentials for two core services:

- **Workers AI** (LLaVA 1.5 7B) - OCR and label field extraction
- **R2 Storage** - Encrypted image storage (S3-compatible)

This guide covers credential setup for development, testing, and pre-production environments.

---

## Quick Start (Development)

### Prerequisites
- Active Cloudflare account
- Access to Cloudflare Dashboard

### Required Credentials
1. **CF_ACCOUNT_ID** - Your Cloudflare account ID
2. **CF_API_TOKEN** - Workers AI API token (scoped)
3. **R2_BUCKET** - R2 bucket name (`label-verify-dev`)
4. **R2_ACCESS_KEY** - R2 access key ID
5. **R2_SECRET_KEY** - R2 secret access key
6. **R2_ENDPOINT** - R2 endpoint URL

### Setup Steps

#### 1. Get Your Account ID

1. Log in to [Cloudflare Dashboard](https://dash.cloudflare.com)
2. Navigate to any domain or Workers & Pages
3. Find **Account ID** in the right sidebar under "API"
4. Copy the account ID

#### 2. Create Workers AI API Token

1. Go to [API Tokens](https://dash.cloudflare.com/profile/api-tokens)
2. Click **Create Token**
3. Select **Create Custom Token**
4. Configure:
   - **Token name**: `label-verify-dev-workers-ai`
   - **Permissions**:
     - Account → Workers AI → Read
     - Account → Account Settings → Read (optional)
   - **Account Resources**: Include → Your account
5. Click **Continue to summary** → **Create Token**
6. **Copy the token immediately** (shown only once)

#### 3. Create R2 Bucket

1. Go to [R2 Dashboard](https://dash.cloudflare.com/r2)
2. Click **Create bucket**
3. Enter bucket name: `label-verify-dev`
4. Location: **Automatic** (recommended)
5. Click **Create bucket**

#### 4. Generate R2 API Token

1. In R2 dashboard, click **Manage R2 API Tokens**
2. Click **Create API token**
3. Configure:
   - **Token name**: `label-verify-dev-r2`
   - **Permissions**: Object Read & Write
   - **Specify buckets**: Apply to specific buckets
   - Select: `label-verify-dev`
   - **TTL** (optional): Set expiration (e.g., 90 days)
4. Click **Create API Token**
5. **Copy both Access Key ID and Secret Access Key** (shown only once)

#### 5. Get R2 Endpoint URL

The endpoint follows this format:
```
https://<account-id>.r2.cloudflarestorage.com
```

Replace `<account-id>` with your Cloudflare account ID from Step 1.

**Example**: `https://abc123def456.r2.cloudflarestorage.com`

#### 6. Configure Environment Variables

Create a `.env` file in the project root:

```bash
# Cloudflare Workers AI
CF_ACCOUNT_ID=your_account_id_here
CF_API_TOKEN=your_workers_ai_token_here

# Cloudflare R2 Storage
R2_BUCKET=label-verify-dev
R2_ACCESS_KEY=your_r2_access_key_here
R2_SECRET_KEY=your_r2_secret_key_here
R2_ENDPOINT=https://your_account_id.r2.cloudflarestorage.com

# Other required credentials
DATABASE_URL=postgresql://localhost:5432/labelverify_dev
REDIS_URL=redis://localhost:6379
ENCRYPTION_KEY=base64_encoded_32_byte_key_here
AZURE_TENANT_ID=your_tenant_id
AZURE_CLIENT_ID=your_client_id
BIND_ADDR=0.0.0.0:3000
```

**Note**: See `.env.example` for a complete template.

#### 7. Verify Setup

Test Workers AI connection:
```bash
curl -X POST \
  https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/ai/run/@cf/llava-hf/llava-1.5-7b-hf \
  -H "Authorization: Bearer $CF_API_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"prompt":"What is in this image?","image":"<base64-encoded-test-image>","max_tokens":100}'
```

Expected response: JSON with `success: true` and model output.

---

## Environment-Specific Setup

### Development Environment

**Purpose**: Local development and manual testing

**Bucket Name**: `label-verify-dev`

**Configuration**:
- Use `.env` file (gitignored)
- Shared credentials OK for small teams
- Enable CORS on R2 bucket for local testing

**Best Practices**:
- Keep credentials in `.env` file only
- Never commit `.env` to git
- Use scoped API tokens (minimum permissions)
- Consider separate Cloudflare account for isolation

**Estimated Cost**: ~$5-10/month (light usage)

---

### Testing Environment (CI/CD)

**Purpose**: Automated integration tests in CI/CD pipelines

**Bucket Name**: `label-verify-test`

**Setup Steps**:

1. Create separate R2 bucket: `label-verify-test`
2. Generate dedicated R2 API token scoped to test bucket
3. Store credentials as CI/CD secrets:

**GitHub Actions**:
```yaml
# .github/workflows/test.yml
env:
  CF_ACCOUNT_ID: ${{ secrets.CF_ACCOUNT_ID }}
  CF_API_TOKEN: ${{ secrets.CF_API_TOKEN }}
  R2_BUCKET: label-verify-test
  R2_ACCESS_KEY: ${{ secrets.R2_ACCESS_KEY_TEST }}
  R2_SECRET_KEY: ${{ secrets.R2_SECRET_KEY_TEST }}
  R2_ENDPOINT: ${{ secrets.R2_ENDPOINT }}
```

**GitLab CI**:
```yaml
# .gitlab-ci.yml
variables:
  R2_BUCKET: label-verify-test

test:
  variables:
    CF_ACCOUNT_ID: $CF_ACCOUNT_ID
    CF_API_TOKEN: $CF_API_TOKEN
    R2_ACCESS_KEY: $R2_ACCESS_KEY_TEST
    R2_SECRET_KEY: $R2_SECRET_KEY_TEST
```

**Best Practices**:
- Use separate bucket to avoid contaminating dev data
- Implement lifecycle policy to auto-delete test objects >7 days
- Consider mocking Workers AI in unit tests (use real API only for integration)
- Rotate tokens quarterly
- Never print secrets in CI logs

**Data Cleanup**:

Set R2 lifecycle policy to auto-delete old test data:
1. Go to R2 bucket settings
2. Add lifecycle rule:
   - Name: `auto-delete-old-tests`
   - Filter: All objects
   - Action: Delete after 7 days

**Estimated Cost**: ~$1-5/month

---

### Pre-Production (Staging)

**Purpose**: Production-like environment for final validation

**Bucket Name**: `label-verify-staging`

**Setup Steps**:

1. Create R2 bucket: `label-verify-staging`
2. Configure bucket settings to mirror production:
   - Encryption: Enabled
   - CORS: Match production rules
   - Lifecycle: Retain data 30 days
3. Generate production-strength API tokens
4. Store credentials in secret management system:
   - AWS Secrets Manager
   - HashiCorp Vault
   - Azure Key Vault
   - Kubernetes Secrets

**Infrastructure as Code Example (Terraform)**:

```hcl
resource "cloudflare_r2_bucket" "staging" {
  account_id = var.cloudflare_account_id
  name       = "label-verify-staging"
  location   = "auto"
}

resource "aws_secretsmanager_secret" "cf_staging" {
  name = "label-verify/staging/cloudflare"
}

resource "aws_secretsmanager_secret_version" "cf_staging" {
  secret_id = aws_secretsmanager_secret.cf_staging.id
  secret_string = jsonencode({
    CF_ACCOUNT_ID = var.cloudflare_account_id
    CF_API_TOKEN  = var.cloudflare_api_token_staging
    R2_BUCKET     = cloudflare_r2_bucket.staging.name
    R2_ACCESS_KEY = var.r2_access_key_staging
    R2_SECRET_KEY = var.r2_secret_key_staging
    R2_ENDPOINT   = var.r2_endpoint
  })
}
```

**Best Practices**:
- Mirror production configuration exactly
- Use separate Cloudflare account for isolation (recommended)
- Implement data retention policies
- Test failover scenarios (rate limits, quota exhaustion)
- Rotate credentials quarterly

**Estimated Cost**: ~$15-60/month (depends on traffic simulation)

---

## Security Best Practices

### API Token Scoping

✅ **DO**:
- Use scoped API tokens (not global API keys)
- Apply principle of least privilege
- Create separate tokens per environment
- Enable token expiration (90 days for dev, longer for prod)
- Use token templates for consistency

❌ **DON'T**:
- Never use Global API Key (too broad permissions)
- Don't share tokens across environments
- Don't create tokens without expiration
- Don't grant unnecessary permissions

### Secret Storage

| Environment | Storage Method | Example |
|-------------|----------------|---------|
| Development | `.env` file (gitignored) | Local filesystem |
| CI/CD | Platform secret store | GitHub Secrets, GitLab Variables |
| Staging | Secret management system | AWS Secrets Manager, Vault |
| Production | Secret management system | Azure Key Vault, Vault |

**Critical Rules**:
- ✅ Never commit secrets to git
- ✅ Never hardcode secrets in source code
- ✅ Never print secrets in logs
- ✅ Use secret scanning tools (GitHub secret scanning, GitGuardian)

### Access Control

- Limit who can generate API tokens (use Cloudflare Access)
- Audit token usage regularly (Cloudflare audit logs)
- Revoke unused tokens immediately
- Implement token rotation schedule

### Monitoring

- Enable Cloudflare audit logging
- Monitor API usage for anomalies
- Set up alerts for quota exhaustion
- Track cost per environment

---

## Cost Optimization

### Workers AI

**Pricing**: ~$0.01 per 1,000 inferences (usage-based)

**Optimization Tips**:
1. **Cache OCR results** in PostgreSQL to avoid re-processing
2. **Implement rate limiting** to prevent abuse
3. **Use batch processing** where possible
4. **Set reasonable timeouts** to avoid hanging requests

**Example Caching Strategy**:
```rust
// Check cache before calling Workers AI
if let Some(cached) = db.get_cached_ocr_result(image_hash).await? {
    return Ok(cached);
}

// Call Workers AI only if not cached
let result = workers_ai_client.extract_fields(image).await?;
db.cache_ocr_result(image_hash, &result).await?;
```

### R2 Storage

**Pricing**:
- Storage: $0.015/GB-month
- Operations: Class A (write): $4.50/million, Class B (read): $0.36/million

**Optimization Tips**:
1. **Enable lifecycle policies** to auto-delete old test/dev data
2. **Compress images** before upload (if quality allows)
3. **Use multipart uploads** for large files (>100MB)
4. **Implement client-side caching** to reduce read operations

**Example Lifecycle Policy**:
- Development: Delete objects >30 days old
- Testing: Delete objects >7 days old
- Staging: Delete objects >90 days old

---

## Troubleshooting

### "Invalid API token" Error

**Symptoms**:
```
Error: HTTP 401 - Invalid API token
```

**Solutions**:
1. Verify token has correct permissions (Workers AI → Read)
2. Check token hasn't expired (check expiration date)
3. Ensure using `Bearer` prefix in Authorization header
4. Confirm token is for correct account

**Test Command**:
```bash
curl -X GET \
  https://api.cloudflare.com/client/v4/user/tokens/verify \
  -H "Authorization: Bearer $CF_API_TOKEN"
```

### "Access denied" on R2 Upload

**Symptoms**:
```
Error: 403 Forbidden - Access Denied
```

**Solutions**:
1. Verify R2 token is scoped to correct bucket
2. Check bucket name matches `R2_BUCKET` env var
3. Confirm token has Read+Write permissions (not just Read)
4. Ensure bucket exists in your account

**Debug Steps**:
```bash
# List buckets (requires R2 token with list permission)
curl -X GET \
  https://api.cloudflare.com/client/v4/accounts/$CF_ACCOUNT_ID/r2/buckets \
  -H "Authorization: Bearer $CF_API_TOKEN"
```

### "Account ID not found"

**Symptoms**:
```
Error: 404 - Account not found
```

**Solutions**:
1. Double-check account ID from Cloudflare dashboard
2. Ensure no extra whitespace in env var
3. Verify account ID matches token's account
4. Confirm account has Workers AI enabled

### High Unexpected Costs

**Symptoms**:
- Cloudflare bill much higher than expected
- Sudden spike in Workers AI usage
- R2 storage growing unexpectedly

**Investigation Steps**:
1. Check Cloudflare Analytics for usage spikes
2. Review application logs for retry loops
3. Verify test data cleanup is working
4. Check for leaked credentials (unauthorized usage)
5. Enable rate limiting on API endpoints

**Prevention**:
- Set up billing alerts in Cloudflare dashboard
- Implement usage quotas per environment
- Monitor daily usage during development
- Use separate accounts for isolation

---

## Verification Checklist

### Development Setup
- [ ] Cloudflare account created/accessed
- [ ] R2 bucket `label-verify-dev` created
- [ ] Workers AI API token generated (scoped)
- [ ] R2 API token generated (scoped to dev bucket)
- [ ] `.env` file created in project root
- [ ] All required environment variables set
- [ ] Workers AI connection tested (curl)
- [ ] R2 upload/download tested
- [ ] `.env` confirmed in `.gitignore`

### Testing Setup
- [ ] R2 bucket `label-verify-test` created
- [ ] R2 API token generated (scoped to test bucket)
- [ ] CI/CD secrets configured
- [ ] Lifecycle policy set (auto-delete >7 days)
- [ ] CI/CD pipeline tested with real credentials
- [ ] Secret scanning enabled

### Pre-Production Setup
- [ ] R2 bucket `label-verify-staging` created
- [ ] Production-strength API tokens generated
- [ ] Credentials stored in secret management system
- [ ] Bucket settings mirrored from production config
- [ ] Monitoring and alerting configured
- [ ] Deployment process documented
- [ ] Token rotation schedule defined

---

## Additional Resources

- [Cloudflare Workers AI Documentation](https://developers.cloudflare.com/workers-ai/)
- [Cloudflare R2 Documentation](https://developers.cloudflare.com/r2/)
- [API Token Permissions Reference](https://developers.cloudflare.com/fundamentals/api/reference/permissions/)
- [R2 Pricing](https://developers.cloudflare.com/r2/pricing/)
- [Workers AI Pricing](https://developers.cloudflare.com/workers-ai/platform/pricing/)

---

## Support

For issues specific to this project, see [CLAUDE.md](../CLAUDE.md).

For Cloudflare-specific issues:
- [Cloudflare Community](https://community.cloudflare.com/)
- [Cloudflare Support](https://support.cloudflare.com/)

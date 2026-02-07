# Documentation

This directory contains comprehensive documentation for the label-verify-hw project.

---

## Getting Started

### New Developers

1. **[CLOUDFLARE_SETUP.md](./CLOUDFLARE_SETUP.md)** - Complete guide to setting up Cloudflare credentials
   - Account ID and API token generation
   - R2 bucket creation
   - Environment-specific configuration
   - Security best practices
   - Troubleshooting

2. **[QUICK_REFERENCE.md](./QUICK_REFERENCE.md)** - Quick reference card
   - Environment variables cheat sheet
   - Common commands
   - Test connectivity
   - Emergency procedures

### DevOps / CI/CD Engineers

- **[CI_CD_SETUP.md](./CI_CD_SETUP.md)** - CI/CD pipeline configuration
  - GitHub Actions
  - GitLab CI
  - CircleCI
  - Jenkins
  - Secret management
  - Cost optimization

---

## Architecture & Design

- **[../CLAUDE.md](../CLAUDE.md)** - Project architecture overview
  - Technology stack
  - Module structure
  - Key dependencies
  - Build commands

---

## Quick Links

### Cloudflare Resources
- [Cloudflare Dashboard](https://dash.cloudflare.com)
- [API Tokens](https://dash.cloudflare.com/profile/api-tokens)
- [R2 Dashboard](https://dash.cloudflare.com/r2)
- [Workers AI Documentation](https://developers.cloudflare.com/workers-ai/)
- [R2 Documentation](https://developers.cloudflare.com/r2/)

### Testing
- Test R2 connectivity: `cargo run --example test_r2`
- Test Workers AI: `cargo run --example test_workers_ai`
- Run all tests: `cargo test`

---

## Common Workflows

### Setting Up Development Environment

```bash
# 1. Clone repository
git clone <repo-url>
cd label-verify-hw

# 2. Copy environment template
cp .env.example .env

# 3. Follow setup guide to get credentials
# See: docs/CLOUDFLARE_SETUP.md

# 4. Edit .env with your credentials
nano .env

# 5. Test connectivity
cargo run --example test_r2
cargo run --example test_workers_ai

# 6. Run the server
cargo run
```

### Adding New CI/CD Platform

1. Read [CI_CD_SETUP.md](./CI_CD_SETUP.md)
2. Create test R2 bucket: `label-verify-test`
3. Generate scoped R2 API token
4. Add secrets to CI/CD platform
5. Configure pipeline using examples
6. Test with a simple commit

### Rotating Credentials

```bash
# 1. Generate new token in Cloudflare dashboard
# 2. Update .env file (or CI/CD secrets)
# 3. Test connectivity
cargo run --example test_r2

# 4. Revoke old token in dashboard
# 5. Document rotation date
```

---

## Environment Configuration

### Development
- **Bucket**: `label-verify-dev`
- **Storage**: `.env` file (gitignored)
- **Cost**: ~$5-10/month
- **Setup Time**: 15 minutes

### Testing (CI/CD)
- **Bucket**: `label-verify-test`
- **Storage**: CI/CD platform secrets
- **Cost**: ~$1-5/month
- **Setup Time**: 30 minutes

### Staging
- **Bucket**: `label-verify-staging`
- **Storage**: Secret management system (Vault, AWS Secrets Manager)
- **Cost**: ~$15-60/month
- **Setup Time**: 1-2 hours

---

## Security

### Must-Do
- ✅ Never commit `.env` to git
- ✅ Use scoped API tokens (not global keys)
- ✅ Enable secret scanning
- ✅ Rotate credentials quarterly
- ✅ Separate credentials per environment

### Recommended
- Enable Cloudflare audit logging
- Set up billing alerts
- Monitor usage daily during development
- Document emergency procedures
- Back up encryption keys securely

---

## Troubleshooting

For detailed troubleshooting, see:
- [CLOUDFLARE_SETUP.md - Troubleshooting](./CLOUDFLARE_SETUP.md#troubleshooting)
- [QUICK_REFERENCE.md - Troubleshooting](./QUICK_REFERENCE.md#troubleshooting)

### Quick Fixes

| Problem | Solution |
|---------|----------|
| "Invalid API token" | Check token permissions and expiration |
| "Access denied" on R2 | Verify token is scoped to correct bucket |
| Tests pass locally but fail in CI | Check service readiness (postgres, redis) |
| High costs | Review logs for retry loops, enable rate limiting |

---

## Cost Management

### Expected Costs by Environment

| Environment | Workers AI | R2 | Total/Month |
|-------------|-----------|-----|-------------|
| Development | $1-5 | $1 | $5-10 |
| Testing | $1-5 | <$1 | $1-5 |
| Staging | $10-50 | $5-10 | $15-60 |

### Optimization Tips
- Cache OCR results in PostgreSQL
- Implement lifecycle policies (auto-delete old data)
- Use mocks for unit tests (real API for integration only)
- Monitor usage daily during development

---

## Support

### Project-Specific Issues
- Create issue in GitHub repository
- Check existing issues first
- Include error messages and logs

### Cloudflare-Specific Issues
- [Cloudflare Community Forum](https://community.cloudflare.com/)
- [Cloudflare Support](https://support.cloudflare.com/)
- [Cloudflare Status Page](https://www.cloudflarestatus.com/)

---

## Contributing

When adding documentation:
1. Keep it concise and actionable
2. Include code examples where helpful
3. Test all commands before documenting
4. Update this README if adding new docs

---

## Document Status

| Document | Last Updated | Status |
|----------|--------------|--------|
| CLOUDFLARE_SETUP.md | 2026-02-07 | ✅ Complete |
| CI_CD_SETUP.md | 2026-02-07 | ✅ Complete |
| QUICK_REFERENCE.md | 2026-02-07 | ✅ Complete |
| README.md (this file) | 2026-02-07 | ✅ Complete |

---

## Feedback

Found an issue with the documentation?
- Unclear instructions
- Missing information
- Outdated content
- Broken links

Please open an issue or submit a pull request!

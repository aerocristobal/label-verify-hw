# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in label-verify-hw, please report it responsibly.

### How to Report

1. **Do NOT** create a public GitHub issue for security vulnerabilities.
2. Email your findings to: **security@labelverify.example.com**
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: Within 48 hours of your report
- **Assessment**: Within 5 business days, we will assess severity and impact
- **Resolution**: Critical vulnerabilities will be patched within 14 days
- **Disclosure**: We follow a 90-day coordinated disclosure policy

### Scope

The following are in scope for security reports:

- Authentication and authorization bypass
- SQL injection or other injection attacks
- Cross-site scripting (XSS)
- Encryption weaknesses (AES-256-GCM implementation)
- Insecure storage of label images or credentials
- API key exposure
- Denial of service vulnerabilities
- Dependency vulnerabilities (critical/high severity)

### Out of Scope

- Social engineering attacks
- Physical security
- Issues in third-party services (Cloudflare, PostgreSQL, Redis)
- Vulnerabilities requiring physical access

### Security Best Practices

This project implements the following security measures:

- **Encryption at rest**: AES-256-GCM for all stored label images
- **TLS in transit**: All external API calls use HTTPS/TLS 1.2+
- **Input validation**: garde-based validation on all user inputs
- **Image validation**: Format verification before processing
- **Non-root containers**: Docker images run as unprivileged user
- **Dependency auditing**: Regular `cargo audit` checks
- **Secret management**: Environment-based configuration (never hardcoded)

### Acknowledgments

We appreciate security researchers who help keep this project safe. Responsible reporters will be credited (with permission) in our release notes.

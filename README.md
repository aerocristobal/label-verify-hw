# label-verify-hw

TTB (Alcohol and Tobacco Tax and Trade Bureau) label verification system using Cloudflare Workers AI for automated OCR and compliance checking.

## Features

- **Secure Upload**: HTTPS endpoint with multipart upload, image validation, and AES-256-GCM encryption
- **Cloudflare Integration**: Uses Workers AI (LLaVA 1.5 7B) for OCR and R2 for encrypted storage
- **Async Processing**: Redis-backed job queue for scalable background processing
- **TTB Validation**: Automated compliance checking against TTB regulations
- **Fast Queries**: PostgreSQL for job tracking and results retrieval

## Architecture

```
┌─────────┐     Upload      ┌────────────┐     Queue      ┌────────┐
│ Client  │ ─────────────> │ API Server │ ─────────────> │ Redis  │
└─────────┘   (Encrypted)   └────────────┘                └────────┘
                                  │                            │
                                  ▼                            ▼
                            ┌──────────┐               ┌──────────┐
                            │  R2 (S3) │               │  Worker  │
                            │ Encrypted│ <──────────── │ Process  │
                            │  Storage │               └──────────┘
                            └──────────┘                      │
                                                              ▼
                                                        ┌──────────┐
                                                        │ Workers  │
                                                        │ AI (OCR) │
                                                        └──────────┘
                                                              │
                                                              ▼
                                                        ┌──────────┐
                                                        │ Postgres │
                                                        │ Results  │
                                                        └──────────┘
```

## Quick Start

### Prerequisites

- Rust 1.75+ (edition 2024)
- PostgreSQL 15+
- Redis 7+
- Cloudflare account with Workers AI and R2 enabled

### 1. Clone and Setup

```bash
git clone <repo-url>
cd label-verify-hw
cp .env.example .env
```

### 2. Configure Cloudflare Credentials

See [docs/CLOUDFLARE_SETUP.md](docs/CLOUDFLARE_SETUP.md) for detailed instructions.

Quick summary:
```bash
# In .env file:
CF_ACCOUNT_ID=your_account_id
CF_API_TOKEN=your_workers_ai_token
R2_BUCKET=label-verify-dev
R2_ACCESS_KEY=your_r2_access_key
R2_SECRET_KEY=your_r2_secret_key
R2_ENDPOINT=https://your_account_id.r2.cloudflarestorage.com
```

### 3. Setup Local Services

```bash
# Start PostgreSQL (via Docker)
docker run -d \
  -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=labelverify_dev \
  --name postgres \
  postgres:15

# Start Redis (via Docker)
docker run -d \
  -p 6379:6379 \
  --name redis \
  redis:7

# Update .env with connection strings
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/labelverify_dev
REDIS_URL=redis://localhost:6379
```

### 4. Generate Encryption Key

```bash
openssl rand -base64 32
# Add to .env as ENCRYPTION_KEY
```

### 5. Run Database Migrations

Migrations run automatically when the server starts, but you can run them manually:

```bash
cargo install sqlx-cli
sqlx migrate run
```

### 6. Start the API Server

```bash
cargo run
```

Server will start on `http://0.0.0.0:3000`

### 7. Start the Worker (in a separate terminal)

```bash
cargo run --bin worker
```

## API Endpoints

### POST /api/v1/verify

Upload a label image for verification.

**Request**:
```bash
curl -X POST http://localhost:3000/api/v1/verify \
  -F "image=@label.jpg" \
  -F "brand_name=Test Wine" \
  -F "class_type=Wine" \
  -F "expected_abv=13.5"
```

**Response**:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "message": "Label submitted for verification"
}
```

### GET /api/v1/verify/{job_id}

Check job status and retrieve results.

**Request**:
```bash
curl http://localhost:3000/api/v1/verify/550e8400-e29b-41d4-a716-446655440000
```

**Response**:
```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "result": {
    "passed": true,
    "confidence_score": 0.95,
    "field_results": [
      {
        "field_name": "brand_name",
        "expected": "Test Wine",
        "extracted": "Test Wine",
        "matches": true,
        "similarity_score": 1.0
      }
    ]
  },
  "error": null
}
```

### GET /health

Health check endpoint.

```bash
curl http://localhost:3000/health
```

## Development

### Build

```bash
cargo build
```

### Run Tests

```bash
# Unit tests
cargo test

# Integration tests (requires running PostgreSQL and Redis)
cargo test --test integration_test -- --ignored
```

### Lint

```bash
cargo clippy
```

### Format

```bash
cargo fmt
```

### Security Audit

```bash
cargo audit
```

## Project Structure

```
label-verify-hw/
├── src/
│   ├── main.rs                 # API server entry point
│   ├── app_state.rs            # Shared application state
│   ├── config/                 # Environment configuration
│   ├── db/                     # Database module
│   │   ├── mod.rs              # Connection pool
│   │   └── queries.rs          # SQL queries
│   ├── models/                 # Data models
│   │   ├── job.rs              # VerificationJob
│   │   ├── label.rs            # ExtractedLabelFields
│   │   └── verification.rs     # Request/response types
│   ├── routes/                 # HTTP handlers
│   │   ├── health.rs           # Health check
│   │   └── verify.rs           # Verification endpoints
│   ├── services/               # Business logic
│   │   ├── encryption.rs       # AES-256-GCM
│   │   ├── ocr.rs              # Workers AI client
│   │   ├── queue.rs            # Redis job queue
│   │   ├── storage.rs          # R2 client
│   │   └── validation.rs       # TTB compliance
│   └── bin/
│       └── worker.rs           # Background job processor
├── migrations/                 # Database migrations
├── tests/                      # Integration tests
├── examples/                   # Testing examples
├── docs/                       # Documentation
│   ├── CLOUDFLARE_SETUP.md     # Setup guide
│   ├── CI_CD_SETUP.md          # CI/CD configuration
│   └── QUICK_REFERENCE.md      # Quick reference
├── Cargo.toml                  # Dependencies
├── CLAUDE.md                   # Project guidance
└── README.md                   # This file
```

## Testing Cloudflare Connectivity

### Test R2 Storage

```bash
cargo run --example test_r2
```

### Test Workers AI

```bash
cargo run --example test_workers_ai
```

## Documentation

- [Cloudflare Setup Guide](docs/CLOUDFLARE_SETUP.md) - Complete setup instructions
- [CI/CD Setup](docs/CI_CD_SETUP.md) - Automated testing configuration
- [Quick Reference](docs/QUICK_REFERENCE.md) - Command cheat sheet
- [Project Architecture](CLAUDE.md) - Technical details

## Security

- **Encryption at Rest**: All images encrypted with AES-256-GCM before storage
- **In-Memory Decryption**: Images decrypted only in worker memory, never persisted unencrypted
- **Scoped API Tokens**: Cloudflare tokens with minimum required permissions
- **Input Validation**: Image format, size, and content validation
- **Structured Logging**: Security events tracked (no sensitive data logged)

## Performance

- **Upload Response**: < 3 seconds (includes encryption and R2 upload)
- **Status Query**: < 200ms (database query only)
- **OCR Processing**: ~5 seconds (Workers AI inference)
- **End-to-End**: < 10 seconds (upload to results)

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## Support

- **Issues**: GitHub Issues
- **Documentation**: `docs/` directory
- **Cloudflare Support**: https://support.cloudflare.com/

## Roadmap

See [GitHub Issues](https://github.com/aerocristobal/label-verify-hw/issues) for planned features and enhancements.

### Phase 1 (MVP) - ✅ Complete
- [x] US-013: Secure HTTPS Upload Endpoint
- [x] US-014: Workers AI OCR Processing
- [x] US-015: Job Status and Results Retrieval
- [x] Database schema and migrations
- [x] Integration tests

### Phase 2 (Enhanced)
- [ ] US-002: Government Warning Verification
- [ ] US-003: Intelligent Case Matching
- [ ] US-007: Audit Trail and Review History
- [ ] US-012: Name and Address Verification
- [ ] US-016: Encryption Key Management and Rotation
- [ ] US-017: Rate Limiting and DDoS Protection
- [ ] US-018: Audit Logging and Compliance Tracking

### Phase 3 (Scale)
- [ ] US-005: Batch Label Processing
- [ ] US-006: Robust Image Quality Handling
- [ ] US-009: Agent Learning and System Feedback
- [ ] OpenSSF OSPS Baseline compliance

## Acknowledgments

- Cloudflare for Workers AI and R2 infrastructure
- Rust community for excellent async ecosystem
- TTB for regulatory guidance

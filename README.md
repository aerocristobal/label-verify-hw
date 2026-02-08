# label-verify-hw

TTB (Alcohol and Tobacco Tax and Trade Bureau) label verification system. Automates beverage label compliance checking using Cloudflare Workers AI for OCR and the authoritative TTB COLA public database for reference validation.

## Features

- **AI-Powered OCR**: Cloudflare Workers AI (LLaVA 1.5 7B) extracts brand, class/type, ABV, net contents, and more from label images
- **TTB COLA Integration**: Read-through cache queries the [TTB COLA public database](https://www.ttbonline.gov/colasonline/publicSearchColasBasic.do) on cache miss for authoritative label data
- **TTB Compliance Validation**: Checks against 27 CFR standards of identity, ABV tolerance (±0.3%), category ABV ranges, same-field-of-vision requirements, and mandatory field presence
- **Database-Backed Matching**: Fuzzy matching (Jaro-Winkler) against cached beverages with match history tracking
- **Encrypted Storage**: AES-256-GCM encryption at rest for all uploaded images in Cloudflare R2
- **Async Processing**: Redis-backed job queue with background worker processing
- **Compliance UI**: Web interface with expandable detail panels, compliance status indicators, and validation type attribution
- **Observability**: Structured JSON logging, Prometheus metrics endpoint

## Architecture

```
┌─────────┐     Upload      ┌────────────┐     Queue      ┌────────┐
│ Client  │ ───-──────────> │ API Server │ ─────────────> │ Redis  │
│ (Web UI)│    (Encrypted)  └────────────┘                └────────┘
└─────────┘                       │                            │
                                  ▼                            ▼
                            ┌──────────┐               ┌──────────┐
                            │  R2 (S3) │               │  Worker  │
                            │ Encrypted│ <──────────── │ Process  │
                            │  Storage │               └──────────┘
                            └──────────┘                     │
                                                     ┌───────┴───────┐
                                                     ▼               ▼
                                               ┌──────────┐   ┌──────────┐
                                               │ Workers  │   │ TTB COLA │
                                               │ AI (OCR) │   │ Public DB│
                                               └──────────┘   └──────────┘
                                                     │               │
                                                     └───────┬───────┘
                                                             ▼
                                                       ┌──────────┐
                                                       │ Postgres │
                                                       │ Results  │
                                                       └──────────┘
```

### Verification Flow

1. Client uploads label image via web UI or API
2. API encrypts image, stores in R2, enqueues job in Redis
3. Worker dequeues job, downloads and decrypts image
4. Workers AI (LLaVA) extracts label fields via OCR
5. Validation engine checks fields against:
   - Local beverage cache (exact match by brand + class)
   - TTB COLA public database (on cache miss, results cached for future lookups)
   - TTB standards of identity (27 CFR Parts 4, 5, 7)
   - Category ABV ranges (wine: 5-24%, spirits: 30-95%, beer: 0.5-15%)
6. Results stored in PostgreSQL, client polls for completion

## Quick Start

### Prerequisites

- Docker and Docker Compose
- Cloudflare account with Workers AI and R2 enabled

### 1. Clone and Configure

```bash
git clone https://github.com/aerocristobal/label-verify-hw.git
cd label-verify-hw
cp .env.prod.example .env.prod
# Edit .env.prod with your credentials
```

### 2. Start the Stack

```bash
docker compose --env-file .env.prod up -d
```

This starts PostgreSQL, Redis, the API server (port 3000), and the background worker. Migrations run automatically on startup.

### 3. Verify

```bash
curl http://localhost:3000/health
```

Open `http://localhost:3000` for the web UI.

### Local Development (without Docker)

```bash
# Prerequisites: Rust 1.75+, PostgreSQL 15+, Redis 7+
cp .env.example .env
cargo run            # API server on 0.0.0.0:3000
cargo run --bin worker  # Background worker (separate terminal)
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/` | Web UI (embedded HTML) |
| `GET` | `/health` | Health check (database + Redis status) |
| `GET` | `/metrics` | Prometheus metrics |
| `POST` | `/api/v1/verify` | Submit label image for verification |
| `GET` | `/api/v1/verify/{job_id}` | Get job status and results |

### Submit Verification

```bash
curl -X POST http://localhost:3000/api/v1/verify \
  -F "image=@label.jpg"
```

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "pending",
  "message": "Label submitted for verification"
}
```

### Get Results

```bash
curl http://localhost:3000/api/v1/verify/550e8400-e29b-41d4-a716-446655440000
```

```json
{
  "job_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "completed",
  "result": {
    "passed": true,
    "confidence_score": 0.95,
    "match_type": "ttb_cola_lookup",
    "match_confidence": 0.87,
    "field_results": [
      {
        "field_name": "ttb_cola_reference",
        "expected": "HARVEYS — DESSERT FLAVORED WINE (TTB ID: 21322001000891)",
        "extracted": "Harveys — Distilled Spirits",
        "matches": true,
        "similarity_score": 0.87
      },
      {
        "field_name": "abv_ttb_cola_reference",
        "expected": "18.0% (inferred from TTB class: DESSERT FLAVORED WINE)",
        "extracted": "15.0%",
        "matches": true,
        "similarity_score": 0.97
      }
    ],
    "warnings": []
  }
}
```

## Project Structure

```
label-verify-hw/
├── src/
│   ├── main.rs                    # Axum server, routes, middleware
│   ├── lib.rs                     # Library exports
│   ├── app_state.rs               # Shared application state
│   ├── config/mod.rs              # AppConfig (env-based via envy)
│   ├── models/
│   │   ├── job.rs                 # VerificationJob, JobStatus
│   │   ├── label.rs               # ExtractedLabelFields, VerificationResult
│   │   ├── beverage.rs            # KnownBeverage, BeverageCategoryRule
│   │   └── verification.rs        # Request/response types
│   ├── routes/
│   │   ├── health.rs              # GET /health
│   │   ├── metrics.rs             # GET /metrics (Prometheus)
│   │   └── verify.rs              # POST + GET /api/v1/verify
│   ├── services/
│   │   ├── encryption.rs          # AES-256-GCM encrypt/decrypt
│   │   ├── ocr.rs                 # Workers AI LLaVA client
│   │   ├── queue.rs               # Redis job queue
│   │   ├── storage.rs             # R2 upload/download/delete
│   │   ├── validation.rs          # TTB compliance + database matching
│   │   ├── ttb_standards.rs       # 27 CFR standards of identity
│   │   └── ttb_cola.rs            # TTB COLA public database client
│   ├── db/
│   │   ├── mod.rs                 # Connection pool + migration runner
│   │   ├── queries.rs             # Job CRUD queries
│   │   └── beverage_queries.rs    # Beverage lookup + TTB COLA upsert
│   └── bin/
│       └── worker.rs              # Background job processor
├── static/
│   └── index.html                 # Web UI (embedded at compile time)
├── migrations/                    # PostgreSQL migrations (auto-run)
├── scripts/                       # Python utilities (seeding, TTB queries)
├── tests/                         # E2E + integration tests with 9 label images
├── docs/                          # Extended documentation
├── Dockerfile.api                 # Multi-stage API build
├── Dockerfile.worker              # Multi-stage worker build
├── docker-compose.yml             # Full stack (postgres, redis, api, worker)
├── Cargo.toml                     # Rust dependencies
└── CLAUDE.md                      # AI assistant project guidance
```

## Database Schema

| Table | Purpose |
|-------|---------|
| `verification_jobs` | Job tracking: status, image key, extracted fields, results |
| `known_beverages` | Beverage reference cache (TTB COLA, manual sources) |
| `beverage_category_rules` | TTB-compliant ABV ranges per category (wine, spirits, beer) |
| `beverage_match_history` | Match analytics: type, confidence, ABV deviation per job |

## Validation Checks

The system performs multi-layer validation on each label:

| Check | Source | Description |
|-------|--------|-------------|
| Brand name match | Database | Jaro-Winkler fuzzy matching (threshold: 0.85) |
| Class/type validity | 27 CFR | TTB standards of identity with spelling correction |
| ABV tolerance | 27 CFR | ±0.3% for exact matches, ±3.0% for TTB-inferred |
| Category ABV range | Database | Wine 5-24%, spirits 30-95%, beer 0.5-15% |
| TTB COLA reference | TTB public DB | Authoritative label approval data |
| Same field of vision | 27 CFR 5.63 | Brand, class, and ABV must appear together |
| Mandatory fields | 27 CFR | Brand, class/type, ABV, net contents required |
| Net contents format | TTB | Valid volume with metric unit |

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `REDIS_URL` | Yes | Redis connection string |
| `CF_ACCOUNT_ID` | Yes | Cloudflare account ID |
| `CF_API_TOKEN` | Yes | Cloudflare Workers AI API token |
| `R2_BUCKET` | Yes | R2 storage bucket name |
| `R2_ACCESS_KEY` | Yes | R2 access key |
| `R2_SECRET_KEY` | Yes | R2 secret key |
| `R2_ENDPOINT` | Yes | R2 endpoint URL |
| `ENCRYPTION_KEY` | Yes | Base64-encoded 32-byte AES key |
| `BIND_ADDR` | No | Server bind address (default: `0.0.0.0:3000`) |
| `RUST_LOG` | No | Log level filter (default: `info`) |

## Docker Deployment

```bash
# Build and start
docker compose --env-file .env.prod up -d --build

# Scale workers
docker compose --env-file .env.prod up -d --scale worker=3

# View logs
docker logs labelverify-api -f
docker logs labelverify-worker -f

# Run migration manually
docker exec labelverify-postgres psql -U labelverify -d labelverify_prod \
  -f /migrations/20260208001_add_ttb_cola_match_type.sql
```

## Development

```bash
cargo build              # Build
cargo test               # Unit tests
cargo clippy             # Lint
cargo fmt                # Format
cargo audit              # Security audit
cargo run --example test_r2          # Test R2 connectivity
cargo run --example test_workers_ai  # Test Workers AI connectivity
```

## Security

- **Encryption at rest**: AES-256-GCM for all stored images
- **In-memory decryption**: Images never persisted unencrypted
- **Scoped API tokens**: Cloudflare tokens with minimum permissions
- **Input validation**: Image format (JPEG/PNG/WebP), size (1KB-10MB), field validation via garde
- **Compile-time query checking**: sqlx verifies SQL at build time
- **Structured logging**: Security events tracked, no sensitive data logged

See [SECURITY.md](SECURITY.md) for vulnerability reporting.

## Performance

| Operation | Typical Latency |
|-----------|----------------|
| Upload + encryption + R2 storage | < 3 seconds |
| OCR (Workers AI inference) | ~5 seconds |
| TTB COLA database query | ~1 second |
| Validation + database matching | < 0.5 seconds |
| Status query | < 200ms |
| **End-to-end** (upload to results) | **~7-10 seconds** |

## Documentation

| Document | Description |
|----------|-------------|
| [Cloudflare Setup](docs/CLOUDFLARE_SETUP.md) | Workers AI and R2 configuration |
| [Deployment Guide](docs/CLOUDFLARE_DEPLOYMENT.md) | Production deployment |
| [TTB COLA Integration](docs/TTB_COLA_INTEGRATION.md) | TTB public database client details |
| [Database Implementation](DATABASE_IMPLEMENTATION.md) | Schema design and queries |
| [E2E Testing](docs/E2E_TESTING_IMPLEMENTATION.md) | End-to-end test suite |
| [Troubleshooting](docs/TROUBLESHOOTING.md) | Common issues and solutions |
| [Quick Reference](docs/QUICK_REFERENCE.md) | Command cheat sheet |
| [CI/CD Setup](docs/CI_CD_SETUP.md) | GitHub Actions configuration |

## Roadmap

See [GitHub Issues](https://github.com/aerocristobal/label-verify-hw/issues) for all user stories.

### Phase 1: MVP — Complete

- [x] US-001: Basic Label Field Verification
- [x] US-004: Simple, Accessible User Interface
- [x] US-008: Network-Compatible Architecture
- [x] US-010: Performance Monitoring and Reliability
- [x] US-011: Beverage Classification Validation
- [x] US-013: Secure HTTPS Upload Endpoint
- [x] US-014: Workers AI OCR Processing
- [x] US-015: Job Status and Results Retrieval
- [x] US-021: OpenSSF OSPS Baseline - Documentation

### Phase 2: Enhanced — In Progress

- [x] US-002: Government Warning Verification
- [x] US-003: Intelligent Case Matching
- [x] US-025: Clear Mismatch Source Attribution
- [x] US-026: Database-Backed Beverage Reference
- [x] US-029: Improved Verification Results Display
- [x] US-030: TTB COLA Read-Through Cache
- [ ] US-007: Audit Trail and Review History
- [ ] US-012: Name and Address Verification
- [ ] US-016: Encryption Key Management and Rotation
- [ ] US-017: Rate Limiting and DDoS Protection
- [ ] US-018: Audit Logging and Compliance Tracking
- [ ] US-026: Session-Based Job History
- [ ] US-027: Automatic Job Result Expiration
- [ ] US-028: Mobile Camera Integration

### Phase 3: Scale

- [ ] US-005: Batch Label Processing
- [ ] US-006: Robust Image Quality Handling
- [ ] US-009: Agent Learning and System Feedback
- [ ] US-019: OpenSSF OSPS Baseline - Access Control
- [ ] US-020: OpenSSF OSPS Baseline - Build & Release Security
- [ ] US-022: OpenSSF OSPS Baseline - Legal & Licensing
- [ ] US-023: OpenSSF OSPS Baseline - Quality & Version Control
- [ ] US-024: OpenSSF OSPS Baseline - Vulnerability Management

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| axum 0.8 | Web framework (multipart uploads) |
| tokio | Async runtime |
| reqwest | HTTP client (Workers AI, TTB COLA) |
| sqlx 0.8 | Async PostgreSQL (compile-time checked) |
| rust-s3 | S3-compatible R2 storage |
| aes-gcm | AES-256-GCM encryption |
| scraper | HTML parsing (TTB COLA responses) |
| garde | Derive-macro input validation |
| strsim | Jaro-Winkler fuzzy matching |
| redis | Async job queue |
| tracing | Structured logging |
| metrics | Prometheus observability |

## License

MIT License - see [LICENSE](LICENSE) for details.

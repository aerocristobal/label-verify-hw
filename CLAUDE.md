# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

label-verify-hw — TTB (Alcohol and Tobacco Tax and Trade Bureau) label verification system. Licensed under MIT.

## Architecture

Single Rust + Axum codebase that:
- Calls **Cloudflare Workers AI** (LLaVA 1.5 7B) via REST for OCR/field extraction from beverage label images
- Uses **Cloudflare R2** (S3-compatible) for encrypted image storage
- Uses **PostgreSQL** (via sqlx) for job/verification metadata
- Uses **Redis** for async job queue
- Applies **AES-256-GCM** encryption for images at rest
- Validates with **garde** (derive-macro validation) and **strsim** (fuzzy matching)

## Build Commands

```bash
cargo build          # Build the project
cargo run            # Run the server (binds 0.0.0.0:3000)
cargo test           # Run tests
cargo audit          # Check dependencies for vulnerabilities
cargo clippy         # Lint
```

## Module Structure

```
src/
├── main.rs              # Axum server setup, routes, middleware
├── config/mod.rs        # AppConfig (env-based via envy)
├── models/
│   ├── job.rs           # VerificationJob, JobStatus
│   ├── label.rs         # ExtractedLabelFields, BeverageClass, VerificationResult
│   └── verification.rs  # Request/response types (VerifyRequest, VerifyResponse)
├── routes/
│   ├── health.rs        # GET /health
│   └── verify.rs        # POST /api/v1/verify, GET /api/v1/verify/{job_id}
└── services/
    ├── encryption.rs    # AES-256-GCM encrypt/decrypt
    ├── ocr.rs           # Workers AI LLaVA client
    ├── queue.rs         # Redis job queue (enqueue/dequeue/complete)
    ├── storage.rs       # R2 client (upload/download/delete)
    └── validation.rs    # Fuzzy matching + TTB field verification
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| axum 0.8 | Web framework (multipart uploads) |
| tokio | Async runtime |
| tower-http | CORS, compression, request limits, tracing |
| reqwest | HTTP client for Workers AI |
| rust-s3 | S3-compatible R2 access |
| sqlx 0.8 | Async PostgreSQL (compile-time checked queries) |
| aes-gcm | AES-256-GCM encryption |
| garde | Derive-macro input validation |
| strsim | Jaro-Winkler fuzzy matching |
| strum | Enum <-> string for TTB codes |
| redis | Async job queue |
| tracing | Structured logging |
| metrics + prometheus exporter | Observability |

## Environment Variables

The server expects these env vars (or a `.env` file):
- `BIND_ADDR` — Server address (default: 0.0.0.0:3000)
- `DATABASE_URL` — PostgreSQL connection string
- `REDIS_URL` — Redis connection string
- `CF_ACCOUNT_ID` / `CF_API_TOKEN` — Cloudflare Workers AI
- `R2_BUCKET` / `R2_ACCESS_KEY` / `R2_SECRET_KEY` / `R2_ENDPOINT` — R2 storage
- `ENCRYPTION_KEY` — Base64-encoded 32-byte AES key
- `AZURE_TENANT_ID` / `AZURE_CLIENT_ID` — Azure AD JWT validation

## Audit Notes

- `cargo audit` reports RUSTSEC-2023-0071 (rsa crate) — false positive; pulled into Cargo.lock by sqlx-mysql but NOT compiled (we only use postgres)
- RUSTSEC-2025-0134 (rustls-pemfile unmaintained) — transitive via rust-s3, warning only

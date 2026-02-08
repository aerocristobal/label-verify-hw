# End-to-End Testing Guide

This directory contains end-to-end tests for the label verification system using real label images.

## Test Images

The `tests/` directory contains 9 test label images (`test_label1.png` through `test_label9.png`) ranging from 789KB to 4.5MB in size.

## Test Structure

```
tests/
├── README.md              # This file
├── fixtures/
│   └── mod.rs            # Test fixtures with expected OCR values
├── helpers/
│   └── mod.rs            # Test helper utilities (upload, poll, assert)
├── e2e_test.rs           # Main E2E test suite
├── integration_test.rs   # Existing integration tests
└── test_label*.png       # Test label images (9 files)
```

## Prerequisites

To run the E2E tests, you need:

1. **PostgreSQL Database** - Running with migrations applied
2. **Redis** - Running for job queue
3. **API Server** - Running on configured port (default: 3000)
4. **Worker Process** - Running to process verification jobs
5. **Cloudflare Credentials** - Workers AI and R2 configured in `.env`

## Setup

### 1. Extract OCR Results from Test Images

First, extract the actual label data from test images using Workers AI:

```bash
# Run OCR extraction on all test images
cargo run --example extract_test_labels > test_ocr_results.md

# This will generate fixture data that you can copy to tests/fixtures/mod.rs
```

### 2. Update Test Fixtures

Copy the generated fixtures from the OCR extraction output to `tests/fixtures/mod.rs`:

```rust
pub const TEST_FIXTURES: &[TestLabelFixture] = &[
    TestLabelFixture {
        filename: "test_label1.png",
        expected_brand: "Stone Creek",  // From OCR
        expected_class: "Wine",
        expected_abv: 13.5,
        expected_net_contents: "750 mL",
        should_pass: true,
        description: "Valid wine label - 1.7MB",
    },
    // ... more fixtures
];
```

### 3. Seed Database with TTB COLA Data (Optional)

Query the TTB COLA database for matching beverage records:

```bash
# Install Python dependencies
pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv

# Query and cache COLA records for brands in test images
python3 scripts/query_ttb_cola.py --brand "Stone Creek" --cache
python3 scripts/query_ttb_cola.py --brand "Jack Daniels" --cache
# ... repeat for each brand found in OCR extraction
```

**Note:** The TTB COLA website may require manual navigation. The script provides a framework but may need adaptation.

### 4. Start Infrastructure

```bash
# Start PostgreSQL and Redis (if using Docker)
docker compose up -d postgres redis

# Run database migrations
sqlx migrate run

# Start API server (in one terminal)
cargo run --bin api

# Start worker (in another terminal)
cargo run --bin worker
```

## Running Tests

### Health Check Test

Verify that the API server is running:

```bash
cargo test --test e2e_test test_e2e_health_check -- --ignored --nocapture
```

### Single Label Test

Test verification with one image:

```bash
cargo test --test e2e_test test_e2e_single_label_verification -- --ignored --nocapture
```

### All Test Labels

Run verification on all 9 test images:

```bash
cargo test --test e2e_test test_e2e_all_test_labels -- --ignored --nocapture
```

### Large Image Handling

Test that 4.5MB images are properly resized:

```bash
cargo test --test e2e_test test_e2e_large_image_handling -- --ignored --nocapture
```

### Invalid Image Rejection

Test that malformed images are rejected:

```bash
cargo test --test e2e_test test_e2e_image_format_validation -- --ignored --nocapture
```

### Concurrent Uploads

Test handling of multiple simultaneous uploads:

```bash
cargo test --test e2e_test test_e2e_concurrent_uploads -- --ignored --nocapture
```

### Run All E2E Tests

```bash
cargo test --test e2e_test -- --ignored --nocapture --test-threads=1
```

**Note:** Use `--test-threads=1` to run tests sequentially for more reliable results.

## Configuration

### Environment Variables

E2E tests use these environment variables:

- `API_BASE_URL` - Base URL for API server (default: `http://localhost:3000`)
- `DATABASE_URL` - PostgreSQL connection string
- `REDIS_URL` - Redis connection string
- `CF_ACCOUNT_ID` - Cloudflare account ID
- `CF_API_TOKEN` - Cloudflare API token
- `R2_BUCKET` / `R2_ENDPOINT` / `R2_ACCESS_KEY` / `R2_SECRET_KEY` - R2 storage config
- `ENCRYPTION_KEY` - AES-256 encryption key (base64-encoded)

### Override API URL

To test against a remote server:

```bash
API_BASE_URL=https://api.example.com cargo test --test e2e_test -- --ignored
```

## Test Workflow

Each E2E test follows this workflow:

1. **Upload** - POST multipart form with image to `/api/v1/verify`
2. **Poll** - GET `/api/v1/verify/{job_id}` until status is `completed` or `failed`
3. **Validate** - Parse `VerificationResult` and assert expected values

## Troubleshooting

### Tests Timeout

If tests timeout waiting for job completion:

- Check that the worker process is running
- Check Redis connection (worker polls jobs from Redis)
- Check Workers AI connectivity (OCR might be slow)
- Increase timeout in `helpers/mod.rs` `poll_job_status()` function

### OCR Extraction Fails

If OCR consistently fails:

- Check Cloudflare API credentials
- Verify Workers AI quota/limits
- Check image format and size
- Review logs for Workers AI errors

### Database Connection Errors

If database queries fail:

- Ensure PostgreSQL is running
- Verify `DATABASE_URL` is correct
- Run migrations: `sqlx migrate run`
- Check that `known_beverages` table exists

### Verification Always Fails

If all verifications fail:

- Check that test fixtures have correct expected values (from OCR)
- Verify validation logic in `src/services/validation.rs`
- Review fuzzy matching thresholds (may need tuning)
- Check database seeding (known beverages may be missing)

## CI/CD Integration

See `.github/workflows/test.yml` for automated E2E testing in GitHub Actions.

The workflow:
1. Starts PostgreSQL and Redis services
2. Runs database migrations
3. Seeds test data from TTB COLA
4. Starts API server and worker in background
5. Runs E2E tests with appropriate timeouts

## Next Steps

1. **Extract OCR Results** - Run `cargo run --example extract_test_labels`
2. **Update Fixtures** - Copy generated data to `tests/fixtures/mod.rs`
3. **Seed Database** - Query TTB COLA and cache records
4. **Run Tests** - Start infrastructure and run E2E test suite
5. **Review Results** - Analyze failures and tune validation logic

## Additional Tools

### Discover Test Images

Inspect test images without running OCR:

```bash
bash scripts/discover_test_labels.sh
```

### Manual API Testing

Test API manually with curl:

```bash
# Upload image
curl -X POST http://localhost:3000/api/v1/verify \
  -F "image=@tests/test_label1.png" \
  -F "brand_name=Stone Creek" \
  -F "class_type=Wine" \
  -F "expected_abv=13.5"

# Check status (replace {job_id} with response from above)
curl http://localhost:3000/api/v1/verify/{job_id}
```

## Test Coverage

The E2E test suite covers:

- ✅ Basic verification workflow (upload → process → retrieve)
- ✅ All 9 test images with varying sizes and content
- ✅ Large image handling (4.5MB resizing)
- ✅ Invalid image rejection
- ✅ Concurrent upload handling
- ✅ Database-backed beverage matching
- ✅ Fuzzy matching validation
- ✅ TTB compliance checking

## Contributing

When adding new test images:

1. Add PNG file to `tests/` directory with pattern `test_labelN.png`
2. Run OCR extraction: `cargo run --example extract_test_labels`
3. Add fixture entry to `tests/fixtures/mod.rs`
4. Query TTB COLA for matching record
5. Update test descriptions to explain what scenario is being tested

## License

MIT

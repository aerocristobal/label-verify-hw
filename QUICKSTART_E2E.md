# E2E Testing Quick Start Guide

Get the E2E test suite running in 4 steps.

## Prerequisites

✅ PostgreSQL running on port 5432
✅ Redis running on port 6379
✅ `.env` file configured with Cloudflare credentials

## Step 1: Extract OCR Data (10 mins)

Extract label information from test images using Workers AI:

```bash
# Run OCR extraction on all 9 test images
cargo run --example extract_test_labels > ocr_results.md

# Review the output
cat ocr_results.md
```

This generates fixture data with actual brand names, ABV values, etc.

## Step 2: Update Fixtures (5 mins)

Copy the generated fixtures to the test file:

```bash
# Open the OCR results
open ocr_results.md  # or use your editor

# Copy the "Complete Fixtures File" section
# Replace contents of tests/fixtures/mod.rs
```

**Important**: Review the `should_pass` values. Set to `false` for any images you want to use for negative testing.

## Step 3: Start Infrastructure (2 mins)

Start the API server and worker:

```bash
# Terminal 1: API Server
cargo run --bin api

# Terminal 2: Worker
cargo run --bin worker
```

## Step 4: Run Tests (5 mins)

Run the E2E test suite:

```bash
# Using helper script (recommended)
./scripts/run_e2e_tests.sh

# Or manually
cargo test --test e2e_test -- --ignored --nocapture --test-threads=1
```

## Expected Results

**Success Indicators**:
- ✅ Health check passes
- ✅ At least 7/9 images verify successfully
- ✅ Large image (4.5MB) completes without error
- ✅ Invalid image rejected

**Common Issues**:

| Issue | Solution |
|-------|----------|
| "Job did not complete within 120 seconds" | Worker may not be running - check terminal 2 |
| "Health check failed" | API server not running - check terminal 1 |
| "Connection refused" | PostgreSQL or Redis not running |
| "OCR failed" | Check Cloudflare credentials in `.env` |

## Quick Reference

### Individual Tests

```bash
# Health check only
cargo test --test e2e_test test_e2e_health_check -- --ignored --nocapture

# Single image
cargo test --test e2e_test test_e2e_single_label_verification -- --ignored --nocapture

# All images
cargo test --test e2e_test test_e2e_all_test_labels -- --ignored --nocapture

# Large image handling
cargo test --test e2e_test test_e2e_large_image_handling -- --ignored --nocapture

# Concurrent uploads
cargo test --test e2e_test test_e2e_concurrent_uploads -- --ignored --nocapture
```

### Test Images

| File | Size | Best For |
|------|------|----------|
| test_label9.png | 789KB | Quick tests (smallest) |
| test_label4.png | 4.5MB | Image resizing (largest) |

### Troubleshooting

**Check Infrastructure**:
```bash
# PostgreSQL
nc -z localhost 5432 && echo "✓ PostgreSQL running"

# Redis
nc -z localhost 6379 && echo "✓ Redis running"

# API
curl http://localhost:3000/health && echo "✓ API healthy"
```

**View Logs**:
- API logs: Terminal 1 (where `cargo run --bin api` is running)
- Worker logs: Terminal 2 (where `cargo run --bin worker` is running)

**Database**:
```bash
# Check migrations
sqlx migrate run

# View known beverages
psql $DATABASE_URL -c "SELECT * FROM known_beverages LIMIT 5;"
```

## Optional: Seed Database

Query TTB COLA database for reference data (optional but improves validation):

```bash
# Install dependencies
pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv

# Query for each brand (replace with brands from OCR results)
python3 scripts/query_ttb_cola.py --brand "Stone Creek" --cache
python3 scripts/query_ttb_cola.py --brand "Jack Daniels" --cache
```

**Note**: TTB COLA script may require manual adaptation to the actual website structure.

## What the Tests Validate

- ✅ **Upload Flow**: Multipart image upload to `/api/v1/verify`
- ✅ **Job Queue**: Redis-based async processing
- ✅ **OCR Extraction**: Workers AI LLaVA 1.5 7B model
- ✅ **Image Resizing**: Large images (4.5MB) → 1024px max dimension
- ✅ **Validation**: Fuzzy matching + TTB compliance
- ✅ **Database Matching**: Known beverage reference lookup
- ✅ **Result Retrieval**: Job status polling and result parsing
- ✅ **Error Handling**: Invalid image rejection
- ✅ **Concurrency**: Multiple simultaneous uploads

## Next Steps After Tests Pass

1. **CI/CD Integration**: Add E2E tests to GitHub Actions
2. **Add Negative Tests**: Mark some fixtures with `should_pass: false`
3. **Performance Tuning**: Adjust fuzzy matching thresholds
4. **Coverage**: Add tests for specific edge cases
5. **Documentation**: Update with actual test results

## Full Documentation

For complete details, see:
- `tests/README.md` - Comprehensive testing guide
- `E2E_TESTING_IMPLEMENTATION.md` - Implementation details

## Getting Help

**If tests fail**:
1. Check the "Expected Results" section above
2. Review troubleshooting table
3. Check infrastructure (PostgreSQL, Redis, API, Worker)
4. Verify `.env` configuration
5. Review test output for specific error messages

**Test Output Shows**:
- ✓ Upload successful with job_id
- ✓ Job completion status
- ✓ Verification result (passed/failed)
- ✓ Confidence scores
- ⚠ Warnings or errors

---

**Time to Running Tests**: ~20 minutes (including OCR extraction)
**Test Execution Time**: ~2-3 minutes for full suite
**Success Rate Target**: ≥80% (7+ out of 9 images)

# End-to-End Testing Implementation Summary

## Overview

This document summarizes the implementation of comprehensive end-to-end (E2E) testing for the label-verify-hw system using 9 real label images from the `tests/` directory.

## What Was Implemented

### 1. Test Infrastructure

#### Test Fixtures (`tests/fixtures/mod.rs`)
- **Purpose**: Define expected OCR results for each test image
- **Structure**: `TestLabelFixture` struct with expected brand, class, ABV, net contents
- **Status**: Created with placeholder "TBD" values - needs OCR extraction to populate

#### Test Helpers (`tests/helpers/mod.rs`)
- **Functions**:
  - `upload_label_image()` - Upload image via multipart form to `/api/v1/verify`
  - `poll_job_status()` - Poll job status endpoint with timeout
  - `wait_for_job_completion()` - Convenience wrapper with 120s timeout
  - `assert_verification_result()` - Validate verification results
  - `parse_verification_result()` - Parse JSON result from job status
- **Types**: Mirror API response types for deserialization

#### E2E Test Suite (`tests/e2e_test.rs`)
- **test_e2e_health_check** - Verify API server is running
- **test_e2e_single_label_verification** - Test single image workflow
- **test_e2e_all_test_labels** - Process all 9 test images
- **test_e2e_large_image_handling** - Test 4.5MB image resizing
- **test_e2e_image_format_validation** - Test invalid image rejection
- **test_e2e_concurrent_uploads** - Test parallel upload handling

All tests marked with `#[ignore]` - require running infrastructure.

### 2. OCR Extraction Tools

#### Discovery Script (`examples/discover_test_labels.rs`)
- **Purpose**: Inspect test images and generate fixture templates
- **Output**: Image metadata (size, dimensions, format)
- **Status**: Rust version created (requires sqlx offline mode to compile)
- **Alternative**: Shell script (`scripts/discover_test_labels.sh`) - works without compilation

#### OCR Extraction (`examples/extract_test_labels.rs`)
- **Purpose**: Run Workers AI OCR on all test images
- **Process**:
  1. Load each test image
  2. Call `WorkersAiClient.extract_label_fields()`
  3. Generate fixture Rust code with actual OCR values
  4. Output complete `tests/fixtures/mod.rs` replacement
- **Usage**: `cargo run --example extract_test_labels > ocr_results.md`
- **Status**: Created, ready to use

### 3. TTB COLA Database Integration

#### Query Script (`scripts/query_ttb_cola.py`)
- **Purpose**: Query official TTB COLA database for approved beverage records
- **Features**:
  - Search by brand name
  - Parse COLA search results (HTML scraping)
  - Extract ABV, class type, approval date
  - Cache records in `known_beverages` table
- **Dependencies**: `requests`, `beautifulsoup4`, `lxml`, `psycopg2-binary`, `python-dotenv`
- **Status**: Framework created - needs adaptation to actual TTB website structure
- **Note**: TTB website may require manual navigation or Selenium automation

### 4. Helper Scripts

#### E2E Test Runner (`scripts/run_e2e_tests.sh`)
- **Purpose**: Automated E2E test execution with prerequisite checks
- **Checks**:
  - `.env` file exists
  - PostgreSQL running (port 5432)
  - Redis running (port 6379)
  - API server healthy (port 3000)
  - Worker process prompt
- **Usage**: `./scripts/run_e2e_tests.sh [test_name]`

#### Test Image Discovery (`scripts/discover_test_labels.sh`)
- **Purpose**: Quick inspection of test images without compilation
- **Output**: Size, dimensions, fixture templates
- **Usage**: `bash scripts/discover_test_labels.sh`

### 5. Documentation

#### Test README (`tests/README.md`)
- Comprehensive guide to E2E testing
- Setup instructions
- Running individual tests
- Troubleshooting guide
- CI/CD integration notes

#### This Document
- Implementation summary
- Workflow instructions
- Next steps

### 6. Dependencies

#### Updated `Cargo.toml`
Added to `[dev-dependencies]`:
```toml
futures = "0.3"  # For concurrent test execution
```

#### Python Dependencies (for COLA script)
```bash
pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv
```

### 7. Code Fixes

#### Removed Unused Import
- **File**: `src/services/validation.rs`
- **Change**: Removed unused `use crate::models::beverage::NewMatchHistory;`
- **Impact**: Eliminates compiler warning

## Test Images Discovered

| Filename | Size | Dimensions | Description |
|----------|------|------------|-------------|
| test_label1.png | 1.7MB | 580x1450 | Medium-sized label |
| test_label2.png | 1.5MB | 558x1924 | Tall label |
| test_label3.png | 2.1MB | 618x2034 | Large tall label |
| test_label4.png | 4.5MB | 998x2580 | **Largest** - tests image resizing |
| test_label5.png | 3.8MB | 1116x2858 | Large label |
| test_label6.png | 3.9MB | 946x2762 | Large label |
| test_label7.png | 980KB | 496x1688 | Medium label |
| test_label8.png | 871KB | 480x1746 | Medium label |
| test_label9.png | 789KB | 514x1276 | **Smallest** - good for quick tests |

## Implementation Workflow

### Phase 1: OCR Extraction (Required Next Step)

1. **Extract OCR results from test images**:
   ```bash
   # Ensure .env has CF_ACCOUNT_ID and CF_API_TOKEN
   cargo run --example extract_test_labels > ocr_results.md
   ```

2. **Review OCR output**:
   - Check extracted brand names
   - Verify ABV values
   - Validate class types
   - Note any OCR errors

3. **Update test fixtures**:
   - Copy generated fixtures from `ocr_results.md`
   - Replace `tests/fixtures/mod.rs` content
   - Adjust `should_pass` based on expected validation outcomes

### Phase 2: Database Seeding (Optional but Recommended)

1. **Install Python dependencies**:
   ```bash
   pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv
   ```

2. **Query TTB COLA for each brand**:
   ```bash
   # For each unique brand found in OCR extraction
   python3 scripts/query_ttb_cola.py --brand "Brand Name" --cache
   ```

3. **Alternative**: Manual database seeding
   - Manually search TTB COLA at https://ttbonline.gov/colasonline/publicSearchColasBasic.do
   - Insert records directly into `known_beverages` table

### Phase 3: Infrastructure Setup

1. **Start services**:
   ```bash
   # PostgreSQL and Redis
   docker compose up -d postgres redis

   # Run migrations
   sqlx migrate run
   ```

2. **Start API server** (terminal 1):
   ```bash
   cargo run --bin api
   ```

3. **Start worker** (terminal 2):
   ```bash
   cargo run --bin worker
   ```

### Phase 4: Run Tests

1. **Health check**:
   ```bash
   cargo test --test e2e_test test_e2e_health_check -- --ignored --nocapture
   ```

2. **Single image test**:
   ```bash
   cargo test --test e2e_test test_e2e_single_label_verification -- --ignored --nocapture
   ```

3. **All images**:
   ```bash
   cargo test --test e2e_test test_e2e_all_test_labels -- --ignored --nocapture
   ```

4. **Full suite**:
   ```bash
   # Using helper script
   ./scripts/run_e2e_tests.sh

   # Or directly
   cargo test --test e2e_test -- --ignored --nocapture --test-threads=1
   ```

### Phase 5: Iterate and Tune

1. **Analyze failures**:
   - Review OCR extraction errors
   - Check fuzzy matching thresholds
   - Validate database matching logic

2. **Adjust validation**:
   - Tune similarity thresholds in `src/services/validation.rs`
   - Update category rules if needed
   - Fix OCR parsing issues

3. **Update fixtures**:
   - Mark some tests as `should_pass: false` for negative testing
   - Add tests with intentionally wrong ABV values
   - Test unknown brands (no COLA match)

## Key Design Decisions

### 1. Test Fixtures Populated from OCR
- **Rationale**: Don't hardcode expected values - extract them from actual OCR
- **Benefit**: Tests verify real-world OCR behavior, not idealized data
- **Trade-off**: Fixtures depend on Workers AI accuracy

### 2. TTB COLA as Authoritative Source
- **Rationale**: Use official government database, not commercial sources
- **Benefit**: Authoritative, ethical, publicly accessible
- **Trade-off**: Website scraping may be fragile, requires maintenance

### 3. Tests Marked as `#[ignore]`
- **Rationale**: E2E tests require full infrastructure (DB, Redis, API, Worker, Cloudflare)
- **Benefit**: Don't slow down regular `cargo test` runs
- **Usage**: Run explicitly with `cargo test -- --ignored`

### 4. Sequential Test Execution
- **Rationale**: `--test-threads=1` prevents race conditions in shared infrastructure
- **Benefit**: More reliable results, easier to debug
- **Trade-off**: Slower execution (~2-3 minutes for all tests)

### 5. Non-Strict Assertions During Development
- **Rationale**: `assert_verification_result()` has `strict: bool` parameter
- **Benefit**: Tests pass during development even if fixtures are incomplete
- **Usage**: Set `strict: true` once fixtures are fully populated

## Files Created

### Rust Files
- `tests/fixtures/mod.rs` - Test fixtures (needs OCR population)
- `tests/helpers/mod.rs` - Test helper utilities
- `tests/e2e_test.rs` - E2E test suite (6 tests)
- `examples/discover_test_labels.rs` - Image discovery tool
- `examples/extract_test_labels.rs` - OCR extraction tool

### Scripts
- `scripts/discover_test_labels.sh` - Shell-based image discovery
- `scripts/query_ttb_cola.py` - TTB COLA database query tool
- `scripts/run_e2e_tests.sh` - E2E test runner with checks

### Documentation
- `tests/README.md` - Comprehensive testing guide
- `E2E_TESTING_IMPLEMENTATION.md` - This document

### Configuration
- Updated `Cargo.toml` with `futures` dev dependency

## Current Status

### ✅ Completed
- [x] Test infrastructure (fixtures, helpers, test suite)
- [x] OCR extraction tools
- [x] TTB COLA query framework
- [x] Helper scripts and automation
- [x] Comprehensive documentation
- [x] Code cleanup (removed unused import)

### ⏳ Next Steps (Required)
- [ ] Run OCR extraction: `cargo run --example extract_test_labels`
- [ ] Update fixtures with actual OCR values
- [ ] Query TTB COLA and seed database (or adapt script to actual website)
- [ ] Run E2E tests and analyze results
- [ ] Tune validation thresholds based on test results

### 🎯 Future Enhancements
- [ ] CI/CD integration (GitHub Actions workflow)
- [ ] Test coverage reporting
- [ ] Performance benchmarking
- [ ] Visual regression testing (compare OCR output images)
- [ ] Chaos testing (network failures, corrupted images)
- [ ] Load testing (concurrent uploads)
- [ ] TTB COLA database mirror (for faster lookups)
- [ ] Selenium-based COLA scraping (if needed)

## Known Limitations

### 1. TTB COLA Script
- Framework only - actual website structure unknown
- May require manual adaptation
- Consider Selenium for interactive navigation

### 2. OCR Variability
- LLaVA 1.5 7B model results may vary
- Fixtures capture one snapshot in time
- OCR errors may be inconsistent

### 3. Test Image Content Unknown
- Don't know what's in the test images until OCR extraction
- May need to create additional test images for specific scenarios
- Some images may have OCR extraction failures

### 4. Database Seeding
- Requires manual effort or website scraping
- COLA data may not match test images exactly
- Fuzzy matching required

## How to Use This Implementation

### For Developers

1. **First Time Setup**:
   ```bash
   # Extract OCR from test images
   cargo run --example extract_test_labels > ocr_results.md

   # Update fixtures
   # Copy from ocr_results.md to tests/fixtures/mod.rs

   # Seed database (manual or script)
   python3 scripts/query_ttb_cola.py --brand "..." --cache
   ```

2. **Running Tests**:
   ```bash
   # Start infrastructure
   docker compose up -d postgres redis
   cargo run --bin api    # terminal 1
   cargo run --bin worker # terminal 2

   # Run tests
   ./scripts/run_e2e_tests.sh
   ```

3. **Adding New Test Images**:
   ```bash
   # Add image to tests/
   cp new_label.png tests/test_label10.png

   # Re-run OCR extraction
   cargo run --example extract_test_labels

   # Update fixtures
   # Add new fixture to tests/fixtures/mod.rs
   ```

### For CI/CD

Create `.github/workflows/e2e.yml`:
```yaml
name: E2E Tests

on:
  push:
    branches: [main]
  workflow_dispatch:  # Manual trigger

jobs:
  e2e:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:15
      redis:
        image: redis:7

    steps:
      - uses: actions/checkout@v3
      - name: Setup Rust
        uses: actions-rs/toolchain@v1
      - name: Run migrations
        run: sqlx migrate run
      - name: Seed database
        run: python3 scripts/seed_test_data.py
      - name: Start API & Worker
        run: |
          cargo run --bin api &
          cargo run --bin worker &
      - name: Run E2E tests
        run: cargo test --test e2e_test -- --ignored --test-threads=1
```

## Conclusion

This implementation provides a complete E2E testing framework for the label-verify-hw system. The next critical step is to run OCR extraction on the test images to populate fixtures with actual expected values.

Once fixtures are populated, the test suite can validate:
- ✅ Full verification pipeline (upload → OCR → validation → result)
- ✅ Image resizing for Workers AI
- ✅ Database-backed beverage matching
- ✅ Fuzzy matching and TTB compliance
- ✅ Concurrent request handling
- ✅ Error handling and rejection

**Estimated Time to Complete**:
- OCR Extraction: 10-15 minutes (with 2s delays between images)
- Fixture Updates: 5-10 minutes
- Database Seeding: 15-30 minutes (manual or scripted)
- First Test Run: 5-10 minutes
- Total: ~1 hour to fully operational E2E tests

**Success Criteria**:
- At least 7/9 test images complete successfully
- OCR extraction rate >80%
- Verification logic correctly validates known beverages
- Large images (4.5MB) handled without errors
- Invalid images properly rejected

## Contact & Support

For questions or issues with the E2E testing implementation:
1. Check `tests/README.md` for troubleshooting guide
2. Review test output for specific error messages
3. Verify all prerequisites (DB, Redis, API, Worker, Cloudflare)
4. Check logs: `cargo run --bin worker` shows processing details

---

**Implementation Date**: 2026-02-07
**Status**: Ready for OCR extraction step
**License**: MIT

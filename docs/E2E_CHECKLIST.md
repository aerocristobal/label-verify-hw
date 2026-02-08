# E2E Testing Implementation Checklist

Use this checklist to complete the E2E testing implementation.

## Phase 1: OCR Extraction ⏳

- [ ] Ensure `.env` file has valid Cloudflare credentials
  ```bash
  grep "CF_ACCOUNT_ID" .env
  grep "CF_API_TOKEN" .env
  ```

- [ ] Run OCR extraction on all test images
  ```bash
  cargo run --example extract_test_labels > ocr_results.md
  ```
  **Time**: ~15 minutes (includes 2s delays between images)

- [ ] Review OCR results for accuracy
  - [ ] Check brand names are extracted correctly
  - [ ] Verify ABV values are reasonable (0-100%)
  - [ ] Validate class types (Wine, Distilled Spirits, Malt Beverage)
  - [ ] Note any extraction failures

- [ ] Copy generated fixtures to `tests/fixtures/mod.rs`
  - [ ] Find "Complete Fixtures File" section in `ocr_results.md`
  - [ ] Replace entire contents of `tests/fixtures/mod.rs`
  - [ ] Verify syntax (should compile without errors)

## Phase 2: Database Seeding (Optional) ⏸️

- [ ] Install Python dependencies
  ```bash
  pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv
  ```

- [ ] Query TTB COLA for each unique brand
  - [ ] List brands from OCR extraction
  - [ ] Run query script for each brand:
    ```bash
    python3 scripts/query_ttb_cola.py --brand "Brand Name" --cache
    ```

- [ ] **Alternative**: Manual database seeding
  - [ ] Search TTB COLA website manually
  - [ ] Insert records into `known_beverages` table via SQL

- [ ] Verify database has reference data
  ```bash
  psql $DATABASE_URL -c "SELECT COUNT(*) FROM known_beverages;"
  ```

## Phase 3: Infrastructure Setup ✅

- [ ] PostgreSQL running
  ```bash
  nc -z localhost 5432 && echo "✓ PostgreSQL"
  ```

- [ ] Redis running
  ```bash
  nc -z localhost 6379 && echo "✓ Redis"
  ```

- [ ] Database migrations applied
  ```bash
  sqlx migrate run
  ```

- [ ] Environment variables configured
  - [ ] `DATABASE_URL`
  - [ ] `REDIS_URL`
  - [ ] `CF_ACCOUNT_ID`
  - [ ] `CF_API_TOKEN`
  - [ ] `R2_BUCKET`, `R2_ENDPOINT`, `R2_ACCESS_KEY`, `R2_SECRET_KEY`
  - [ ] `ENCRYPTION_KEY`

## Phase 4: Start Services ⏳

- [ ] Start API server (Terminal 1)
  ```bash
  cargo run --bin api
  ```
  - [ ] Verify starts without errors
  - [ ] Note the port (default: 3000)

- [ ] Start worker (Terminal 2)
  ```bash
  cargo run --bin worker
  ```
  - [ ] Verify connects to Redis
  - [ ] Check for "Polling for jobs..." messages

- [ ] Verify API health
  ```bash
  curl http://localhost:3000/health
  ```
  - [ ] Returns 200 OK
  - [ ] Response indicates healthy status

## Phase 5: Run Tests ⏳

- [ ] Run health check test
  ```bash
  cargo test --test e2e_test test_e2e_health_check -- --ignored --nocapture
  ```
  **Expected**: ✅ PASS

- [ ] Run single label verification
  ```bash
  cargo test --test e2e_test test_e2e_single_label_verification -- --ignored --nocapture
  ```
  **Expected**: ✅ PASS (uses smallest image)

- [ ] Run all test labels
  ```bash
  cargo test --test e2e_test test_e2e_all_test_labels -- --ignored --nocapture
  ```
  **Expected**: ✅ 7-9 images complete successfully

- [ ] Run large image handling test
  ```bash
  cargo test --test e2e_test test_e2e_large_image_handling -- --ignored --nocapture
  ```
  **Expected**: ✅ PASS (4.5MB image resized correctly)

- [ ] Run invalid image test
  ```bash
  cargo test --test e2e_test test_e2e_image_format_validation -- --ignored --nocapture
  ```
  **Expected**: ✅ PASS (rejects invalid image)

- [ ] Run concurrent uploads test
  ```bash
  cargo test --test e2e_test test_e2e_concurrent_uploads -- --ignored --nocapture
  ```
  **Expected**: ✅ PASS (3 images processed in parallel)

- [ ] Run full test suite
  ```bash
  ./scripts/run_e2e_tests.sh
  # or
  cargo test --test e2e_test -- --ignored --nocapture --test-threads=1
  ```
  **Expected**: ✅ All 6 tests pass

## Phase 6: Analysis & Tuning ⏸️

- [ ] Review test output
  - [ ] Note which images passed/failed
  - [ ] Check confidence scores (should be 0.0-1.0)
  - [ ] Review similarity scores for fuzzy matching

- [ ] Analyze failures
  - [ ] OCR extraction errors → Check Workers AI logs
  - [ ] Validation failures → Review matching logic
  - [ ] Timeouts → Check worker processing logs
  - [ ] Database errors → Verify schema and data

- [ ] Tune validation thresholds (if needed)
  - [ ] Adjust `MATCH_THRESHOLD` in `src/services/validation.rs`
  - [ ] Modify fuzzy matching sensitivity
  - [ ] Update category ABV ranges

- [ ] Update test fixtures (if needed)
  - [ ] Mark some tests with `should_pass: false` for negative testing
  - [ ] Add descriptions explaining test purpose
  - [ ] Document any known OCR issues

## Phase 7: Documentation & Cleanup ⏸️

- [ ] Update `tests/README.md` with actual results
  - [ ] Document which images work best
  - [ ] Add troubleshooting notes from experience
  - [ ] Include actual test output examples

- [ ] Document known issues
  - [ ] OCR extraction accuracy by image
  - [ ] Validation edge cases
  - [ ] Performance characteristics

- [ ] Clean up temporary files
  - [ ] `ocr_results.md` (keep for reference or delete)
  - [ ] Any test output logs

- [ ] Commit changes
  ```bash
  git add tests/ examples/ scripts/ Cargo.toml E2E*.md QUICKSTART*.md
  git commit -m "Add comprehensive E2E test suite with real label images"
  ```

## Success Criteria ✅

- [x] **Infrastructure**: All tools and scripts created
- [ ] **OCR Extraction**: 80%+ success rate (7+ of 9 images)
- [ ] **Test Execution**: All 6 tests pass
- [ ] **Large Images**: 4.5MB image processed without errors
- [ ] **Validation**: Fuzzy matching works for known beverages
- [ ] **Concurrency**: Multiple uploads handled correctly
- [ ] **Documentation**: Complete and accurate

## Timeline Estimates

| Phase | Estimated Time | Status |
|-------|----------------|--------|
| 1. OCR Extraction | 15 minutes | ⏳ Next |
| 2. Database Seeding | 30 minutes | ⏸️ Optional |
| 3. Infrastructure Setup | 5 minutes | ✅ Done |
| 4. Start Services | 2 minutes | ⏳ Pending |
| 5. Run Tests | 5 minutes | ⏳ Pending |
| 6. Analysis & Tuning | 20 minutes | ⏸️ After tests |
| 7. Documentation | 10 minutes | ⏸️ After tests |
| **Total** | **~1 hour** | |

## Troubleshooting Quick Reference

| Issue | Quick Fix |
|-------|-----------|
| Cargo won't compile | `export PATH="$HOME/.cargo/bin:$PATH"` |
| sqlx errors | `SQLX_OFFLINE=true` or run migrations first |
| Tests timeout | Check worker is running, increase timeout |
| OCR fails | Verify Cloudflare credentials in `.env` |
| All tests fail | Check API and worker are both running |
| Connection refused | Start PostgreSQL and Redis |

## Next Immediate Steps

**Start here** 👇

1. Run OCR extraction:
   ```bash
   export PATH="$HOME/.cargo/bin:$PATH"
   cargo run --example extract_test_labels > ocr_results.md
   ```

2. Update fixtures:
   ```bash
   # Open ocr_results.md
   # Copy "Complete Fixtures File" section
   # Paste into tests/fixtures/mod.rs
   ```

3. Start services and run tests:
   ```bash
   ./scripts/run_e2e_tests.sh
   ```

## Notes

- Use `--nocapture` to see test output in real-time
- Use `--test-threads=1` to avoid race conditions
- Tests are marked `#[ignore]` - must use `--ignored` flag
- Each test image takes ~10-15 seconds to process (OCR + validation)

---

**Last Updated**: 2026-02-07
**Status**: Ready for OCR extraction
**Completion**: 50% (infrastructure done, needs OCR extraction & testing)

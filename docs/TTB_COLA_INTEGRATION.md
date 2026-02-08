# TTB COLA Integration Documentation

## Overview

This document describes the integration between the Label Verify system and the **TTB COLA (Certificate of Label Approval) Public Registry**, the official U.S. government database of approved alcohol beverage labels.

**Official Source:** https://ttbonline.gov/colasonline/publicSearchColasBasic.do

**Authority:** Alcohol and Tobacco Tax and Trade Bureau (TTB), U.S. Department of the Treasury

## Why TTB COLA?

### Authoritative Data

- **Official Government Database** - TTB COLA is the federal registry of all legally approved alcohol labels in the United States
- **Regulatory Compliance** - All alcohol products sold in the U.S. must have COLA approval
- **Verified Information** - Brand names, class/types, origins are verified by TTB regulators
- **Public Access** - Data is publicly searchable at no cost
- **No Ethical Concerns** - Government data vs. commercial web scraping

### Use Cases for Label Verification

1. **Logical Consistency Validation** - Detect mislabeled products (e.g., wine labeled with 40% ABV)
2. **Brand Verification** - Cross-reference extracted brand names against approved labels
3. **Class/Type Validation** - Verify class/type designations match TTB approvals
4. **Fraud Detection** - Flag products with no COLA approval or inconsistent data

## Architecture

### Database Cache Strategy

The system uses the `known_beverages` PostgreSQL table as a **persistent cache** of TTB COLA data:

```
TTB COLA Website (ttbonline.gov)
         ↓
    [HTTP Client] (Python)
         ↓
  Parse Search Results (BeautifulSoup)
         ↓
  known_beverages table (PostgreSQL cache)
  ├─ brand_name, class_type, abv
  ├─ source: 'ttb_cola'
  ├─ source_url: COLA approval link
  ├─ created_at: cache timestamp
  └─ is_verified: true (official data)
         ↓
  Validation Service (Rust)
  ├─ Check cache first (exact brand + class/type match)
  ├─ Fall back to category rules if no match
  ├─ Warn if cache is stale (>30 days old)
  └─ Return verification result with match metadata
```

### Why Caching?

- **Performance** - TTB COLA website requires HTTP requests (slow, rate-limited)
- **Reliability** - Validation happens in worker (needs fast, local access)
- **Reusability** - Same beverages may be verified repeatedly
- **Staleness Tracking** - Database tracks age of cached data for refresh strategy

## Implementation Components

### 1. TTB COLA Client (Python)

**File:** `scripts/ttb_cola_client.py`

Python HTTP client for querying the TTB COLA public database:

**Key Features:**
- Searches recent approvals by category (wine, distilled spirits, malt beverages)
- Parses HTML results table to extract COLA records
- Infers ABV from class/type descriptions using TTB regulatory ranges
- Maps TTB class/type codes to beverage categories
- Handles pagination and date range filtering

**ABV Inference:**

TTB COLA search results **do not** explicitly list ABV values. The client infers typical ABV based on class/type:

| Class/Type | Inferred ABV | Regulatory Basis |
|------------|--------------|------------------|
| Table Wine | 12.0% | 27 CFR Part 4 (7-14% typical) |
| Dessert Wine | 18.0% | 27 CFR Part 4 (14-24%) |
| Whiskey | 45.0% | 27 CFR Part 5 (40-50% typical) |
| Vodka/Gin | 40.0% | 27 CFR Part 5 (40% standard) |
| Beer/Ale | 5.0% | 27 CFR Part 7 (3-6% typical) |
| Malt Beverage Specialties | 5.0% | 27 CFR Part 7 |

**Usage:**

```bash
# Test the client
python3 scripts/ttb_cola_client.py --category wine --limit 20

# Search all categories
python3 scripts/ttb_cola_client.py --category all --limit 100 --months 12
```

### 2. Cache Seeding Script (Python)

**File:** `scripts/seed_ttb_cola_cache.py`

Populates the `known_beverages` database table with TTB COLA data:

**Features:**
- Queries TTB COLA for recent approvals (configurable date range)
- Infers beverage category and ABV from class/type
- Inserts/updates database records with conflict resolution
- Tracks metadata (COLA ID, approval date, permit number)
- Supports dry-run mode for preview
- Includes refresh mode for stale entries

**Usage:**

```bash
# Install dependencies
pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv

# Seed all categories (100 records each)
python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all

# Seed only wine (last 12 months)
python3 scripts/seed_ttb_cola_cache.py --category wine --months 12

# Dry run (preview without writing)
python3 scripts/seed_ttb_cola_cache.py --limit 10 --category all --dry-run

# Refresh stale entries (>30 days old)
python3 scripts/seed_ttb_cola_cache.py --refresh-stale --stale-days 30
```

**Database Record Structure:**

```sql
INSERT INTO known_beverages (
    brand_name,           -- e.g., "AMBAR ESTATE"
    product_name,         -- e.g., "OLD DOG HAVEN" (fanciful name, if any)
    class_type,           -- e.g., "TABLE RED WINE"
    beverage_category,    -- 'wine', 'distilled_spirits', or 'malt_beverage'
    abv,                  -- Inferred from class/type (e.g., 12.0)
    country_of_origin,    -- e.g., "OREGON", "CALIFORNIA"
    source,               -- 'ttb_cola'
    source_url,           -- https://ttbonline.gov/colasonline/viewColaDetails.do?...
    notes,                -- "COLA ID: 25171001000623, Approved: 09/22/2025, ..."
    is_verified           -- true (official government data)
);
```

### 3. Cache Staleness Tracking (Rust)

**Files:**
- `src/db/beverage_queries.rs` - Database queries with staleness checks
- `src/services/validation.rs` - Validation service with warnings

**New Functions:**

```rust
/// Check if cache entry is stale (older than threshold)
pub fn is_cache_stale(created_at: DateTime<Utc>, threshold_days: i64) -> bool;

/// Find known beverage with staleness information
pub async fn find_known_beverage_with_staleness(
    pool: &PgPool,
    brand: &str,
    class_type: &str,
    staleness_threshold_days: i64,
) -> Result<Option<(KnownBeverage, bool)>, sqlx::Error>;
```

**Staleness Warnings:**

When a cached entry is older than 30 days, the validation service emits a warning:

```json
{
  "warnings": [
    "Database cache entry is older than 30 days. Consider refreshing TTB COLA data for brand 'AMBAR ESTATE' (source: ttb_cola)."
  ]
}
```

**Configuration:**

```rust
// src/services/validation.rs
const CACHE_STALENESS_THRESHOLD_DAYS: i64 = 30;
```

## TTB COLA Website Structure

### Search Form

**URL:** https://ttbonline.gov/colasonline/publicSearchColasBasic.do

**Key Parameters:**

| Parameter | Description | Example |
|-----------|-------------|---------|
| `searchCriteria.dateCompletedFrom` | Start date (MM/DD/YYYY) | `08/01/2025` |
| `searchCriteria.dateCompletedTo` | End date (MM/DD/YYYY) | `02/07/2026` |
| `searchCriteria.productOrFancifulName` | Product/brand name | `STONE CREEK` |
| `searchCriteria.productNameSearchType` | Search type | `B` (brand), `F` (fanciful), `E` (either) |
| `searchCriteria.classTypeFrom` | Class/type code range start | `80` (wine) |
| `searchCriteria.classTypeTo` | Class/type code range end | `89` (wine) |

**Class/Type Code Ranges:**

| Category | Code Range | Examples |
|----------|------------|----------|
| Wine | 80-89 | 80 (Table Red), 81 (Table White), 84 (Sparkling) |
| Distilled Spirits | 100-699 | 101 (Bourbon), 642 (Gin), 166 (Single Malt) |
| Malt Beverages | 900-999 | 900 (Beer), 906 (Flavored Malt Beverage) |

### Results Table

**Columns:**

1. TTB ID - Unique COLA identifier (e.g., `25171001000623`)
2. Permit No. - Facility permit number (e.g., `BWN-OR-21557`)
3. Serial Number - Application serial number
4. Completed Date - Approval date (MM/DD/YYYY)
5. Fanciful Name - Product fanciful name (optional)
6. Brand Name - Brand name (required)
7. Origin - Origin code (numeric)
8. Origin Desc - Origin description (e.g., `CALIFORNIA`, `OREGON`)
9. Class/Type - Class/type code (numeric)
10. Class/Type Desc - Class/type description (e.g., `TABLE RED WINE`)

**Detail Page:**

Each COLA has a detail page accessible via:
```
https://ttbonline.gov/colasonline/viewColaDetails.do?action=publicDisplaySearchBasic&ttbid={TTB_ID}
```

## Data Refresh Strategy

### Manual Refresh

Run the seeding script periodically (e.g., weekly, monthly):

```bash
# Refresh all categories
python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all

# Refresh only stale entries
python3 scripts/seed_ttb_cola_cache.py --refresh-stale --stale-days 30
```

### Automated Refresh (Future Enhancement)

Set up a cron job or scheduled task:

```bash
# Weekly refresh (Monday 2am)
0 2 * * 1 cd /path/to/label-verify-hw && python3 scripts/seed_ttb_cola_cache.py --limit 50 --category all
```

### Staleness Threshold

- **Default:** 30 days
- **Rationale:** TTB COLA approvals don't change frequently; 30 days balances freshness with performance
- **Configurable:** Edit `CACHE_STALENESS_THRESHOLD_DAYS` in `src/services/validation.rs`

## Validation Workflow

### 1. Exact Match

```
User submits label → Extract fields (OCR) → Query database by brand + class/type
                                                        ↓
                                         Found exact match → Check ABV consistency
                                                                      ↓
                                              ABV within ±1% → Pass (high confidence)
                                              ABV differs >1% → Fail (logical inconsistency)
```

### 2. Fuzzy Match

```
User submits label → Extract fields → No exact match → Query by brand only
                                                              ↓
                                                  Found similar brand → Fuzzy match
                                                  No match → Fall back to category rules
```

### 3. Category Rules Fallback

```
No database match → Infer category from class/type → Apply TTB ABV range
                                                            ↓
                                              Wine: 5-24% ABV
                                              Spirits: 30-95% ABV
                                              Beer: 0.5-15% ABV
```

### 4. Staleness Warning

```
Exact match found → Check created_at timestamp → If older than 30 days → Add warning
```

## Troubleshooting

### TTB Website Issues

**Problem:** TTB website returns "No results" for valid searches

**Solutions:**
1. Verify date range is not too broad (max 15 years per TTB)
2. Check class/type code ranges are correct (80-89 for wine, etc.)
3. Try broader date range (6 months instead of 1 month)
4. Remove class/type filter and search all categories

**Problem:** SSL certificate errors

**Solutions:**
- Python client uses `verify=False` to bypass certificate issues
- TTB website has known certificate problems
- Use `urllib3.disable_warnings()` to suppress warnings

### Seeding Script Errors

**Problem:** `ModuleNotFoundError: No module named 'psycopg2'`

**Solution:**
```bash
pip install psycopg2-binary python-dotenv requests beautifulsoup4 lxml
```

**Problem:** Database connection errors

**Solution:**
```bash
# Ensure DATABASE_URL is set in .env file
export DATABASE_URL='postgresql://user:password@localhost:5432/label_verify'

# Or set it temporarily
python3 scripts/seed_ttb_cola_cache.py --limit 10 ...
```

**Problem:** No ABV inferred for certain class types

**Solution:**
- Check `TTBCOLAClient.ABV_RANGES` dictionary in `ttb_cola_client.py`
- Add missing class/type mappings
- Update `infer_abv_from_class_type()` fuzzy matching logic

### Validation Issues

**Problem:** Staleness warnings for fresh data

**Solution:**
- Check `CACHE_STALENESS_THRESHOLD_DAYS` constant in `validation.rs`
- Verify `created_at` timestamps in database are correct
- Run seeding script to refresh entries

**Problem:** No database matches for known brands

**Solution:**
```sql
-- Check if brand exists in database
SELECT brand_name, class_type, abv, source
FROM known_beverages
WHERE LOWER(brand_name) LIKE '%stone%';

-- Check cache age
SELECT brand_name, created_at, NOW() - created_at AS age
FROM known_beverages
WHERE source = 'ttb_cola'
ORDER BY created_at DESC
LIMIT 10;
```

## Testing

### 1. Test TTB COLA Client

```bash
# Test wine search
python3 scripts/ttb_cola_client.py --category wine --limit 10

# Expected: List of 10 wine COLA records with brand names, class types, inferred ABV
```

### 2. Test Seeding Script (Dry Run)

```bash
# Preview what would be imported
python3 scripts/seed_ttb_cola_cache.py --limit 5 --category wine --dry-run

# Expected: Output showing 5 wine records that would be imported
```

### 3. Test Database Cache

```bash
# Seed database
python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all

# Verify records
psql $DATABASE_URL -c "
SELECT source, beverage_category, COUNT(*), AVG(abv)
FROM known_beverages
WHERE source = 'ttb_cola'
GROUP BY source, beverage_category;
"

# Expected: ~300 records across 3 categories
```

### 4. Test Validation with Cache

```bash
# Submit label for known beverage
curl -X POST http://localhost:3000/api/v1/verify \
  -F "image=@test_label_wine.jpg" \
  -F "brand_name=AMBAR ESTATE" \
  -F "class_type=Table Red Wine"

# Expected response:
{
  "job_id": "...",
  "status": "completed",
  "result": {
    "passed": true,
    "matched_beverage_id": "abc-123-def",
    "match_type": "exact",
    "match_confidence": 1.0,
    "abv_deviation": 0.2,
    "warnings": []
  }
}
```

### 5. Test Staleness Warning

```bash
# Manually age a cache entry
psql $DATABASE_URL -c "
UPDATE known_beverages
SET created_at = NOW() - INTERVAL '45 days'
WHERE brand_name = 'AMBAR ESTATE';
"

# Submit verification
curl -X POST http://localhost:3000/api/v1/verify \
  -F "image=@test_label.jpg" \
  -F "brand_name=AMBAR ESTATE"

# Expected: warnings array includes staleness message
```

## ABV Inference Details

### Regulatory Basis

**27 CFR Part 4 (Wine):**
- Table Wine: 7-14% ABV (typical: 11-13%)
- Dessert Wine: 14-24% ABV (typical: 18-20%)
- Sparkling Wine: 10-14% ABV (typical: 12%)

**27 CFR Part 5 (Distilled Spirits):**
- Whiskey: 40% ABV minimum (typical: 40-50%)
- Vodka: 40% ABV minimum (typical: 40%)
- Gin: 40% ABV minimum (typical: 40%)
- Brandy: 40% ABV minimum (typical: 40%)

**27 CFR Part 7 (Malt Beverages):**
- Beer/Ale/Lager: 3-6% ABV (typical: 5%)
- High-Gravity Beer: 7-12% ABV
- Flavored Malt Beverages: 4-6% ABV (typical: 5%)

### Inference Limitations

**Important:** Inferred ABV values are **estimates** based on regulatory ranges, not exact measurements:

1. **No Exact ABV in TTB Results** - TTB COLA search results do not include ABV
2. **Typical Values Used** - Client uses typical/midpoint values for each class/type
3. **Detail Page May Have ABV** - Individual COLA detail pages *may* contain ABV (not currently scraped)
4. **Validation Tolerance** - System allows ±1% deviation for database matches

**Mitigation:**
- Use inferred ABV for **logical consistency checks** (e.g., wine shouldn't be 40% ABV)
- Don't use for **exact ABV validation** (use TTB-mandated ±0.3% tolerance instead)
- Consider scraping COLA detail pages in future for exact ABV (performance trade-off)

## Future Enhancements

### Short-Term

1. **Automated Cache Refresh** - Cron job to refresh stale entries weekly
2. **COLA Detail Page Scraping** - Extract exact ABV, net contents, vintage
3. **Admin UI** - Manage cached beverages, trigger manual refresh
4. **Batch Seeding** - Parallel requests for faster seeding

### Long-Term

1. **TTB API Integration** - Official API if/when TTB provides one
2. **Analytics Dashboard** - Track cache hit rates, match types, staleness
3. **Incremental Seeding** - Only fetch COLAs since last update
4. **Category-Specific Refresh** - Different TTL for wine vs. spirits vs. beer
5. **Machine Learning** - Improve ABV inference from class/type descriptions

## References

- **TTB COLA Public Registry:** https://ttbonline.gov/colasonline/publicSearchColasBasic.do
- **TTB Website:** https://www.ttb.gov
- **27 CFR Part 4 (Wine):** https://www.ecfr.gov/current/title-27/chapter-I/subchapter-A/part-4
- **27 CFR Part 5 (Distilled Spirits):** https://www.ecfr.gov/current/title-27/chapter-I/subchapter-A/part-5
- **27 CFR Part 7 (Malt Beverages):** https://www.ecfr.gov/current/title-27/chapter-I/subchapter-A/part-7
- **TTB COLA FAQs:** http://www.ttb.gov/faqs/colasonline.shtml

## License

MIT License - See LICENSE file in repository root

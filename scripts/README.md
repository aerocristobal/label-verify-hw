# Scripts Directory

## Overview

This directory contains Python scripts for populating and maintaining the beverage reference database used by the Label Verify system.

## Active Scripts

### `ttb_cola_client.py`

TTB COLA (Certificate of Label Approval) public database client.

**Purpose:** Query the official U.S. government TTB COLA database for approved beverage labels.

**Features:**
- Search recent approvals by category (wine, distilled spirits, malt beverages)
- Parse HTML search results
- Infer ABV from class/type descriptions
- Map TTB class/type codes to beverage categories

**Usage:**
```bash
# Test the client
python3 scripts/ttb_cola_client.py --category wine --limit 20

# Search all categories
python3 scripts/ttb_cola_client.py --category all --limit 100 --months 12
```

**Documentation:** See `docs/TTB_COLA_INTEGRATION.md`

---

### `seed_ttb_cola_cache.py`

Database seeding script using TTB COLA as the authoritative data source.

**Purpose:** Populate the `known_beverages` database table with TTB COLA approved beverages as a persistent cache.

**Features:**
- Query TTB COLA for recent approvals
- Infer beverage category and ABV
- Insert/update database with conflict resolution
- Support dry-run and refresh modes
- Track metadata (COLA ID, approval date, permit)

**Usage:**
```bash
# Install dependencies
pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv urllib3

# Seed all categories (100 records each)
python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all

# Seed only wine (last 12 months)
python3 scripts/seed_ttb_cola_cache.py --category wine --months 12

# Dry run (preview without writing)
python3 scripts/seed_ttb_cola_cache.py --limit 10 --category all --dry-run

# Refresh stale entries (>30 days old)
python3 scripts/seed_ttb_cola_cache.py --refresh-stale --stale-days 30
```

**Environment Variables:**
- `DATABASE_URL` - PostgreSQL connection string (required)

**Documentation:** See `docs/TTB_COLA_INTEGRATION.md`

---

## Deprecated Scripts

### `seed_total_wine_data.py` ⚠️ DEPRECATED

**Status:** Replaced by `seed_ttb_cola_cache.py`

**Reason for Deprecation:**
- Used synthetic/mock data, not real Total Wine scraping
- Total Wine is a commercial source, not an official regulatory database
- Ethical concerns about commercial web scraping
- TTB COLA is the authoritative government source

**Migration:**
```bash
# Old approach (deprecated)
python3 scripts/seed_total_wine_data.py --limit 100

# New approach (recommended)
python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all
```

---

## Dependencies

Install all required Python packages:

```bash
pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv urllib3
```

Or use a requirements file:

```bash
# Create requirements.txt
cat > scripts/requirements.txt <<EOF
requests>=2.31.0
beautifulsoup4>=4.12.0
lxml>=5.0.0
psycopg2-binary>=2.9.0
python-dotenv>=1.0.0
urllib3>=2.0.0
EOF

# Install
pip install -r scripts/requirements.txt
```

---

## Common Tasks

### Initial Database Seeding

```bash
# 1. Ensure database is running and migrations are applied
sqlx migrate run

# 2. Seed with TTB COLA data (300 records across all categories)
python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all

# 3. Verify data imported
psql $DATABASE_URL -c "
SELECT source, beverage_category, COUNT(*), AVG(abv)
FROM known_beverages
WHERE source = 'ttb_cola'
GROUP BY source, beverage_category;
"
```

### Refreshing Stale Data

```bash
# Check for stale entries
psql $DATABASE_URL -c "
SELECT COUNT(*), MAX(created_at), MIN(created_at)
FROM known_beverages
WHERE source = 'ttb_cola'
  AND created_at < NOW() - INTERVAL '30 days';
"

# Refresh stale entries
python3 scripts/seed_ttb_cola_cache.py --refresh-stale --stale-days 30
```

### Testing Before Production

```bash
# Dry run to preview what would be imported
python3 scripts/seed_ttb_cola_cache.py --limit 10 --category wine --dry-run

# Import small sample for testing
python3 scripts/seed_ttb_cola_cache.py --limit 5 --category all

# Verify sample data
psql $DATABASE_URL -c "
SELECT brand_name, class_type, abv, source_url
FROM known_beverages
WHERE source = 'ttb_cola'
ORDER BY created_at DESC
LIMIT 10;
"
```

---

## Troubleshooting

### Import Errors

**Problem:** `ModuleNotFoundError: No module named 'psycopg2'`

**Solution:**
```bash
pip install psycopg2-binary
```

**Problem:** `ModuleNotFoundError: No module named 'ttb_cola_client'`

**Solution:**
```bash
# Run from repository root
cd /path/to/label-verify-hw
python3 scripts/seed_ttb_cola_cache.py --limit 10 --category wine
```

### Database Connection Errors

**Problem:** `connection to server at "localhost" failed: password authentication failed`

**Solution:**
```bash
# Check DATABASE_URL is set
echo $DATABASE_URL

# Set it if missing (or add to .env file)
export DATABASE_URL='postgresql://user:password@localhost:5432/label_verify'
```

### TTB Website Errors

**Problem:** `No results found` when searching TTB COLA

**Solutions:**
1. Verify date range is not too broad (max 15 years per TTB)
2. Try broader date range (6+ months)
3. Check class/type code ranges (80-89 for wine, 100-699 for spirits, 900-999 for beer)
4. Remove class/type filter and search all categories

**Problem:** SSL certificate warnings

**Solution:** Scripts use `verify=False` and `urllib3.disable_warnings()` to handle TTB's certificate issues. This is expected behavior.

---

## Related Documentation

- **TTB COLA Integration:** `docs/TTB_COLA_INTEGRATION.md`
- **Database Schema:** `migrations/20260207_002_create_reference_tables.sql`
- **Validation Service:** `src/services/validation.rs`
- **API Documentation:** `README.md`

---

## License

MIT License - See LICENSE file in repository root

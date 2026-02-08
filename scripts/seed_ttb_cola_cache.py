#!/usr/bin/env python3
"""
Seed known_beverages database with TTB COLA approved beverages.

This script queries the TTB COLA public database for recently approved beverage labels
and populates the known_beverages table as a persistent cache.

Usage:
    python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all
    python3 scripts/seed_ttb_cola_cache.py --category wine --months 12 --dry-run
    python3 scripts/seed_ttb_cola_cache.py --limit 50 --refresh-stale

Author: Label Verify System
License: MIT
"""

import argparse
import os
import sys
from datetime import datetime
import psycopg2
from dotenv import load_dotenv

# Import TTB COLA client
from ttb_cola_client import TTBCOLAClient

# Load environment variables
load_dotenv()


def seed_category(db_conn, client: TTBCOLAClient, category: str, limit: int, months: int, dry_run: bool = False):
    """
    Seed database with TTB COLA data for a specific category.

    Args:
        db_conn: PostgreSQL database connection
        client: TTBCOLAClient instance
        category: Beverage category ('wine', 'distilled_spirits', 'malt_beverage')
        limit: Maximum number of records to import
        months: Look back period in months
        dry_run: If True, don't actually insert into database
    """
    print(f"\n{'='*80}")
    print(f"Category: {category.upper().replace('_', ' ')}")
    print(f"{'='*80}\n")

    print(f"🔍 Fetching {limit} recent {category} approvals from TTB COLA...")
    print(f"   Date range: Last {months} months")

    # Fetch recent approvals
    records = client.search_recent_approvals(category=category, limit=limit, months_back=months)

    print(f"\n✅ Found {len(records)} COLA records\n")

    if not records:
        print("⚠️  No records to import")
        return

    cursor = db_conn.cursor()
    imported = 0
    skipped = 0
    updated = 0

    for i, record in enumerate(records, 1):
        brand_name = record['brand_name']
        product_name = record.get('fanciful_name')
        class_type = record['class_type_desc']
        origin = record['origin_desc']
        abv = record.get('inferred_abv')
        ttb_id = record['ttb_id']
        completed_date = record['completed_date']
        source_url = record['source_url']

        # Skip if no ABV could be inferred
        if not abv:
            print(f"⚠️  [{i}/{len(records)}] Skipping {brand_name} - no ABV inferred from class type '{class_type}'")
            skipped += 1
            continue

        # Determine beverage category
        beverage_category = client.get_category_from_class_type(class_type, record['class_type_code'])

        # Build notes field
        notes = f"COLA ID: {ttb_id}, Approved: {completed_date}, Origin: {origin}"
        if record.get('permit_no'):
            notes += f", Permit: {record['permit_no']}"

        if dry_run:
            print(f"✓ [{i}/{len(records)}] Would import: {brand_name} ({class_type}, {abv}% ABV)")
            imported += 1
            continue

        try:
            # Insert or update record
            # Conflict resolution: Update source_url, notes, updated_at if record already exists
            cursor.execute("""
                INSERT INTO known_beverages (
                    brand_name, product_name, class_type, beverage_category,
                    abv, country_of_origin, source, source_url, notes, is_verified
                ) VALUES (
                    %s, %s, %s, %s, %s, %s, 'ttb_cola', %s, %s, true
                )
                ON CONFLICT (LOWER(brand_name), LOWER(COALESCE(product_name, '')), abv)
                DO UPDATE SET
                    source = EXCLUDED.source,
                    source_url = EXCLUDED.source_url,
                    notes = EXCLUDED.notes,
                    updated_at = CURRENT_TIMESTAMP,
                    is_verified = EXCLUDED.is_verified
                RETURNING (xmax = 0) AS inserted
            """, (
                brand_name,
                product_name,
                class_type,
                beverage_category,
                abv,
                origin,
                source_url,
                notes
            ))

            result = cursor.fetchone()
            was_inserted = result[0]

            if was_inserted:
                print(f"✓ [{i}/{len(records)}] Imported: {brand_name} ({class_type}, {abv}% ABV)")
                imported += 1
            else:
                print(f"↻ [{i}/{len(records)}] Updated: {brand_name} ({class_type}, {abv}% ABV)")
                updated += 1

        except Exception as e:
            print(f"✗ [{i}/{len(records)}] Error importing {brand_name}: {e}")
            skipped += 1
            continue

    if not dry_run:
        db_conn.commit()

    print(f"\n{'='*80}")
    print(f"📊 Summary for {category}:")
    print(f"   Imported: {imported}")
    print(f"   Updated: {updated}")
    print(f"   Skipped: {skipped}")
    print(f"{'='*80}\n")


def refresh_stale_entries(db_conn, client: TTBCOLAClient, days_threshold: int = 30, dry_run: bool = False):
    """
    Refresh database entries older than threshold.

    Args:
        db_conn: PostgreSQL database connection
        client: TTBCOLAClient instance
        days_threshold: Age in days to consider stale
        dry_run: If True, don't actually update database
    """
    print(f"\n🔄 Refreshing stale cache entries (older than {days_threshold} days)...\n")

    cursor = db_conn.cursor()

    # Find stale entries
    cursor.execute("""
        SELECT id, brand_name, product_name, class_type, abv
        FROM known_beverages
        WHERE source = 'ttb_cola'
          AND created_at < NOW() - INTERVAL '%s days'
        LIMIT 100
    """, (days_threshold,))

    stale_entries = cursor.fetchall()

    if not stale_entries:
        print("✅ No stale entries found")
        return

    print(f"Found {len(stale_entries)} stale entries\n")

    refreshed = 0
    skipped = 0

    for entry_id, brand_name, product_name, class_type, abv in stale_entries:
        print(f"🔍 Refreshing: {brand_name} ({class_type}, {abv}% ABV)")

        # Search TTB COLA for updated data
        # Note: This is a simplified approach - in production, you'd want more sophisticated matching
        records = client.search_recent_approvals(limit=10, months_back=12)

        # Try to find matching record
        match_found = False
        for record in records:
            if (record['brand_name'].lower() == brand_name.lower() and
                record['class_type_desc'].lower() == class_type.lower()):

                if dry_run:
                    print(f"  ✓ Would refresh with COLA ID {record['ttb_id']}")
                    refreshed += 1
                    match_found = True
                    break

                # Update record
                try:
                    cursor.execute("""
                        UPDATE known_beverages
                        SET source_url = %s,
                            notes = %s,
                            updated_at = CURRENT_TIMESTAMP,
                            created_at = CURRENT_TIMESTAMP
                        WHERE id = %s
                    """, (
                        record['source_url'],
                        f"COLA ID: {record['ttb_id']}, Approved: {record['completed_date']}, Refreshed: {datetime.now().strftime('%Y-%m-%d')}",
                        entry_id
                    ))
                    print(f"  ✓ Refreshed with COLA ID {record['ttb_id']}")
                    refreshed += 1
                    match_found = True
                    break
                except Exception as e:
                    print(f"  ✗ Error refreshing: {e}")
                    skipped += 1
                    break

        if not match_found:
            print(f"  ⚠️  No matching COLA found - keeping existing data")
            skipped += 1

    if not dry_run:
        db_conn.commit()

    print(f"\n📊 Refresh Summary:")
    print(f"   Refreshed: {refreshed}")
    print(f"   Skipped: {skipped}")


def main():
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description='Seed known_beverages database with TTB COLA approved beverages',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Seed all categories with 100 records each
  python3 scripts/seed_ttb_cola_cache.py --limit 100 --category all

  # Seed only wine with last 12 months of approvals
  python3 scripts/seed_ttb_cola_cache.py --category wine --months 12

  # Dry run to preview what would be imported
  python3 scripts/seed_ttb_cola_cache.py --limit 10 --category all --dry-run

  # Refresh stale cache entries
  python3 scripts/seed_ttb_cola_cache.py --refresh-stale --stale-days 30
        """
    )

    parser.add_argument('--category',
                        choices=['wine', 'distilled_spirits', 'malt_beverage', 'all'],
                        default='all',
                        help='Category to seed (default: all)')

    parser.add_argument('--limit',
                        type=int,
                        default=100,
                        help='Records per category to fetch (default: 100)')

    parser.add_argument('--months',
                        type=int,
                        default=6,
                        help='Look back period in months (default: 6)')

    parser.add_argument('--dry-run',
                        action='store_true',
                        help='Preview imports without writing to database')

    parser.add_argument('--refresh-stale',
                        action='store_true',
                        help='Refresh stale cache entries instead of seeding new data')

    parser.add_argument('--stale-days',
                        type=int,
                        default=30,
                        help='Age in days to consider cache stale (default: 30)')

    args = parser.parse_args()

    # Get database URL from environment
    database_url = os.getenv('DATABASE_URL')
    if not database_url:
        print("❌ Error: DATABASE_URL environment variable not set")
        print("   Set it in .env file or export DATABASE_URL='postgresql://...'")
        sys.exit(1)

    # Connect to database
    try:
        print(f"🔌 Connecting to database...")
        db_conn = psycopg2.connect(database_url)
        print(f"✅ Connected to database\n")
    except Exception as e:
        print(f"❌ Error connecting to database: {e}")
        sys.exit(1)

    # Initialize TTB COLA client
    client = TTBCOLAClient()

    if args.dry_run:
        print("🔍 DRY RUN MODE - No changes will be made to database\n")

    try:
        if args.refresh_stale:
            # Refresh stale entries
            refresh_stale_entries(db_conn, client, days_threshold=args.stale_days, dry_run=args.dry_run)
        else:
            # Seed new data
            categories = ['wine', 'distilled_spirits', 'malt_beverage'] if args.category == 'all' else [args.category]

            total_imported = 0
            total_updated = 0
            total_skipped = 0

            for cat in categories:
                seed_category(db_conn, client, cat, args.limit, args.months, dry_run=args.dry_run)

            print(f"\n{'='*80}")
            print(f"✅ Seeding complete!")
            print(f"{'='*80}\n")

            if not args.dry_run:
                # Show summary statistics
                cursor = db_conn.cursor()
                cursor.execute("""
                    SELECT
                        beverage_category,
                        COUNT(*) as count,
                        AVG(abv) as avg_abv
                    FROM known_beverages
                    WHERE source = 'ttb_cola'
                    GROUP BY beverage_category
                    ORDER BY beverage_category
                """)

                print("📊 Database Summary (TTB COLA entries):\n")
                for category, count, avg_abv in cursor.fetchall():
                    print(f"   {category:20s}: {count:4d} records (avg ABV: {avg_abv:.1f}%)")

                cursor.execute("SELECT COUNT(*) FROM known_beverages WHERE source = 'ttb_cola'")
                total = cursor.fetchone()[0]
                print(f"\n   {'Total':20s}: {total:4d} records")

    except KeyboardInterrupt:
        print("\n\n⚠️  Interrupted by user")
        db_conn.rollback()
        sys.exit(1)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        import traceback
        traceback.print_exc()
        db_conn.rollback()
        sys.exit(1)
    finally:
        db_conn.close()


if __name__ == '__main__':
    main()

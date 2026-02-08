#!/usr/bin/env python3
"""
Query TTB COLA database and cache results for beverage reference validation.

The COLA database is the official TTB registry of approved alcohol labels.
This provides authoritative data for validation.

Usage:
    python3 scripts/query_ttb_cola.py --brand "Stone Creek" --class "wine"
    python3 scripts/query_ttb_cola.py --brand "Jack Daniels" --cache

Prerequisites:
    pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv
"""

import argparse
import os
import re
import sys
from typing import Dict, List, Optional

import psycopg2
import requests
from bs4 import BeautifulSoup
from dotenv import load_dotenv

# TTB COLA Public Search URL
TTB_COLA_BASE_URL = "https://ttbonline.gov/colasonline/publicSearchColasBasic.do"


class TTBCOLAClient:
    """Client for querying TTB COLA database."""

    def __init__(self):
        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (compatible; LabelVerifyBot/1.0)'
        })

    def search_by_brand(self, brand_name: str, product_type: Optional[str] = None) -> List[Dict]:
        """
        Search COLA database by brand name.

        Args:
            brand_name: Brand name to search for
            product_type: Optional filter (wine, distilled spirits, malt beverage)

        Returns:
            List of matching COLA records with approval details
        """
        print(f"🔍 Searching TTB COLA database for: {brand_name}")

        # Note: The TTB COLA website structure may change
        # This is a basic example that would need to be adapted to the actual site

        form_data = {
            'publicSearch': 'true',
            'searchType': 'basic',
            'brandName': brand_name,
        }

        if product_type:
            form_data['productType'] = product_type

        try:
            response = self.session.post(TTB_COLA_BASE_URL, data=form_data, timeout=30)
            response.raise_for_status()

            return self._parse_search_results(response.text)

        except requests.RequestException as e:
            print(f"❌ Error querying TTB COLA: {e}")
            return []

    def _parse_search_results(self, html: str) -> List[Dict]:
        """Parse HTML search results into structured data."""
        soup = BeautifulSoup(html, 'html.parser')
        results = []

        # Note: This parsing logic is illustrative
        # The actual TTB website structure would need to be inspected
        # and this code adapted accordingly

        # Look for result tables
        result_rows = soup.find_all('tr', class_='resultRow')

        if not result_rows:
            # Try alternative selectors
            result_rows = soup.select('table.results tr')

        for row in result_rows:
            cols = row.find_all('td')
            if len(cols) < 3:
                continue

            # Extract data from columns (structure depends on actual website)
            result = {
                'cola_id': cols[0].get_text(strip=True) if len(cols) > 0 else '',
                'brand_name': cols[1].get_text(strip=True) if len(cols) > 1 else '',
                'class_type': cols[2].get_text(strip=True) if len(cols) > 2 else '',
                'approval_date': cols[3].get_text(strip=True) if len(cols) > 3 else '',
            }

            # Extract additional details if available
            details_link = row.find('a', href=True)
            if details_link:
                result['details_url'] = details_link['href']

            if result['cola_id']:  # Only add if we have a COLA ID
                results.append(result)

        return results


def extract_abv_from_text(text: str) -> Optional[float]:
    """Extract ABV percentage from text."""
    match = re.search(r'(\d+(?:\.\d+)?)\s*%?\s*(?:ABV|Alcohol|alc)', text, re.IGNORECASE)
    return float(match.group(1)) if match else None


def infer_category(class_type: str) -> str:
    """Map class/type to beverage category."""
    lower = class_type.lower()
    if 'wine' in lower:
        return 'wine'
    elif any(s in lower for s in ['whiskey', 'vodka', 'gin', 'rum', 'brandy', 'spirit', 'tequila', 'cognac']):
        return 'distilled_spirits'
    elif any(s in lower for s in ['beer', 'ale', 'lager', 'malt', 'stout', 'porter']):
        return 'malt_beverage'
    return 'wine'  # default


def cache_cola_record(db_conn, cola_record: Dict):
    """Cache COLA record in known_beverages table."""
    cursor = db_conn.cursor()

    # Extract ABV from class_type or notes if present
    abv = extract_abv_from_text(cola_record.get('class_type', ''))

    category = infer_category(cola_record['class_type'])

    try:
        cursor.execute("""
            INSERT INTO known_beverages (
                brand_name, product_name, class_type, beverage_category,
                abv, source, source_url, notes, is_verified
            ) VALUES (
                %s, %s, %s, %s, %s, 'ttb_cola', %s, %s, true
            )
            ON CONFLICT (LOWER(brand_name), LOWER(COALESCE(product_name, '')), COALESCE(abv, 0))
            DO UPDATE SET
                class_type = EXCLUDED.class_type,
                source_url = EXCLUDED.source_url,
                notes = EXCLUDED.notes,
                updated_at = CURRENT_TIMESTAMP
        """, (
            cola_record['brand_name'],
            None,  # product_name - not typically in COLA search results
            cola_record['class_type'],
            category,
            abv,
            cola_record.get('details_url'),
            f"COLA ID: {cola_record['cola_id']}, Approved: {cola_record.get('approval_date', 'Unknown')}"
        ))

        db_conn.commit()
        print(f"   ✅ Cached: {cola_record['brand_name']} ({category})")

    except Exception as e:
        print(f"   ⚠️  Error caching record: {e}")
        db_conn.rollback()


def main():
    parser = argparse.ArgumentParser(description='Query TTB COLA database')
    parser.add_argument('--brand', required=True, help='Brand name to search')
    parser.add_argument('--class', dest='class_type', choices=['wine', 'spirits', 'beer'],
                        help='Product class filter')
    parser.add_argument('--cache', action='store_true', help='Cache results in database')
    args = parser.parse_args()

    # Load environment variables
    load_dotenv()

    client = TTBCOLAClient()

    print(f"\n{'='*60}")
    print(f"TTB COLA Database Query")
    print(f"{'='*60}\n")

    results = client.search_by_brand(args.brand, args.class_type)

    if not results:
        print("\n⚠️  No COLA records found")
        print("\nNote: The TTB COLA website requires interactive navigation.")
        print("This script provides a framework for automation, but you may need to:")
        print("1. Manually search at: https://ttbonline.gov/colasonline/publicSearchColasBasic.do")
        print("2. Adapt the parsing logic to match the current website structure")
        print("3. Consider using a browser automation tool like Selenium")
        return

    print(f"\n✅ Found {len(results)} COLA records:\n")

    for idx, record in enumerate(results, 1):
        print(f"{idx}. COLA ID: {record.get('cola_id', 'N/A')}")
        print(f"   Brand: {record.get('brand_name', 'N/A')}")
        print(f"   Class: {record.get('class_type', 'N/A')}")
        print(f"   Approved: {record.get('approval_date', 'N/A')}")
        if 'details_url' in record:
            print(f"   URL: {record['details_url']}")
        print()

    if args.cache and results:
        database_url = os.getenv('DATABASE_URL')
        if not database_url:
            print("❌ DATABASE_URL not set, cannot cache results")
            sys.exit(1)

        print("💾 Caching results to database...\n")

        try:
            db_conn = psycopg2.connect(database_url)
            for record in results:
                cache_cola_record(db_conn, record)

            print(f"\n✅ Successfully cached {len(results)} records")
            db_conn.close()

        except Exception as e:
            print(f"\n❌ Database error: {e}")
            sys.exit(1)

    print(f"\n{'='*60}")
    print("Query complete")
    print(f"{'='*60}\n")


if __name__ == '__main__':
    main()

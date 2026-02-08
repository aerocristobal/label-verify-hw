#!/usr/bin/env python3
"""
TTB COLA Public Registry Client

Queries the TTB (Alcohol and Tobacco Tax and Trade Bureau) COLA (Certificate of Label Approval)
public database to retrieve approved beverage labels.

Official Source: https://ttbonline.gov/colasonline/publicSearchColasBasic.do

Author: Label Verify System
License: MIT
"""

import requests
from bs4 import BeautifulSoup
from datetime import datetime, timedelta
from typing import List, Dict, Optional
import re
import urllib3

# Suppress SSL warnings for TTB website (has certificate issues)
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)


class TTBCOLAClient:
    """Client for querying TTB COLA public database."""

    # TTB ABV ranges by class/type (based on 27 CFR regulations)
    ABV_RANGES = {
        # Wine (27 CFR Part 4)
        'TABLE RED WINE': 12.0,
        'TABLE WHITE WINE': 12.0,
        'TABLE WINE': 11.0,
        'SPARKLING WINE/CHAMPAGNE': 12.0,
        'DESSERT /PORT/SHERRY/(COOKING) WINE': 18.0,
        'DESSERT WINE': 18.0,

        # Distilled Spirits (27 CFR Part 5)
        'STRAIGHT BOURBON WHISKY': 45.0,
        'KENTUCKY STRAIGHT BOURBON WHISKEY': 45.0,
        'AMERICAN SINGLE MALT WHISKEY': 43.0,
        'GIN': 40.0,
        'GIN SPECIALTIES': 40.0,
        'VODKA': 40.0,
        'RUM': 40.0,
        'TEQUILA': 40.0,
        'BRANDY': 40.0,
        'OTHER SPECIALTIES & PROPRIETARIES': 40.0,

        # Malt Beverages (27 CFR Part 7)
        'BEER': 5.0,
        'MALT BEVERAGES SPECIALITIES - FLAVORED': 5.0,
        'ALE': 5.5,
        'LAGER': 5.0,
        'STOUT': 6.0,
        'IPA': 6.5,
    }

    def __init__(self):
        """Initialize TTB COLA client."""
        self.base_url = "https://ttbonline.gov/colasonline"
        self.search_url = f"{self.base_url}/publicSearchColasBasicProcess.do"
        self.detail_url = f"{self.base_url}/viewColaDetails.do"

        self.session = requests.Session()
        self.session.headers.update({
            'User-Agent': 'Mozilla/5.0 (compatible; LabelVerifyBot/1.0; +https://github.com/aerocristobal/label-verify-hw)'
        })

    def search_recent_approvals(
        self,
        category: Optional[str] = None,
        limit: int = 100,
        months_back: int = 6
    ) -> List[Dict]:
        """
        Search for recently approved COLAs.

        Args:
            category: Filter by category ('wine', 'distilled_spirits', 'malt_beverage')
                     If None, returns all categories
            limit: Maximum number of records to return (default: 100)
            months_back: Look back period in months (default: 6)

        Returns:
            List of COLA records with structure:
            {
                'ttb_id': '25171001000623',
                'permit_no': 'BWN-OR-21557',
                'serial_number': '250014',
                'completed_date': '09/22/2025',
                'fanciful_name': 'OLD DOG HAVEN',
                'brand_name': 'AMBAR ESTATE',
                'origin_code': '38',
                'origin_desc': 'OREGON',
                'class_type_code': '80',
                'class_type_desc': 'TABLE RED WINE',
                'source_url': 'https://...',
                'inferred_abv': 12.0
            }
        """
        # Calculate date range
        to_date = datetime.now()
        from_date = to_date - timedelta(days=months_back * 30)

        # Build search parameters
        params = {
            'searchCriteria.dateCompletedFrom': from_date.strftime('%m/%d/%Y'),
            'searchCriteria.dateCompletedTo': to_date.strftime('%m/%d/%Y'),
            'searchCriteria.productOrFancifulName': '',
            'searchCriteria.productNameSearchType': 'E',  # Either brand or fanciful
        }

        # Category-specific class type filters
        # Note: TTB uses numeric codes for class/type, ranges vary by category
        if category == 'wine':
            params['searchCriteria.classTypeFrom'] = '80'
            params['searchCriteria.classTypeTo'] = '89'
        elif category == 'distilled_spirits':
            params['searchCriteria.classTypeFrom'] = '100'
            params['searchCriteria.classTypeTo'] = '699'
        elif category == 'malt_beverage':
            params['searchCriteria.classTypeFrom'] = '900'
            params['searchCriteria.classTypeTo'] = '999'

        try:
            # Submit search
            response = self.session.post(
                self.search_url + '?action=search',
                data=params,
                verify=False,  # TTB website has certificate issues
                timeout=30
            )

            response.raise_for_status()

            # Parse results
            return self._parse_search_results(response.text, limit)

        except Exception as e:
            print(f"Error searching TTB COLA: {e}")
            return []

    def _parse_search_results(self, html: str, limit: int) -> List[Dict]:
        """
        Parse HTML search results table.

        TTB results table structure:
        TTB ID | Permit No. | Serial Number | Completed Date | Fanciful Name | Brand Name | Origin | Origin Desc | Class/Type | Class/Type Desc
        """
        soup = BeautifulSoup(html, 'html.parser')

        # Find results table - look for table containing "TTB ID" header
        results_table = None
        for table in soup.find_all('table'):
            # Check if this table has the expected headers
            text = table.get_text()
            if 'TTB ID' in text and 'Brand Name' in text and 'Class/Type' in text:
                results_table = table
                break

        if not results_table:
            # Check for "No results" message
            if 'No results were found' in html:
                print("⚠️  No results found for search criteria")
            return []

        records = []

        # Extract table rows
        rows = results_table.find_all('tr')

        # Skip header row, process data rows
        for row in rows[1:]:
            cells = row.find_all('td')

            # Expected: 10 cells per row
            if len(cells) < 10:
                continue

            # Extract cell values
            ttb_id = cells[0].get_text(strip=True)
            permit_no = cells[1].get_text(strip=True)
            serial_number = cells[2].get_text(strip=True)
            completed_date = cells[3].get_text(strip=True)
            fanciful_name = cells[4].get_text(strip=True)
            brand_name = cells[5].get_text(strip=True)
            origin_code = cells[6].get_text(strip=True)
            origin_desc = cells[7].get_text(strip=True)
            class_type_code = cells[8].get_text(strip=True)
            class_type_desc = cells[9].get_text(strip=True)

            # Skip if missing critical fields
            if not ttb_id or not brand_name or not class_type_desc:
                continue

            # Extract link to detail page
            link = cells[0].find('a')
            detail_url = None
            if link and link.get('href'):
                href = link.get('href')
                if not href.startswith('http'):
                    detail_url = f"{self.base_url}/{href}"
                else:
                    detail_url = href
            else:
                detail_url = f"{self.detail_url}?action=publicDisplaySearchBasic&ttbid={ttb_id}"

            # Infer ABV from class type description
            inferred_abv = self.infer_abv_from_class_type(class_type_desc)

            record = {
                'ttb_id': ttb_id,
                'permit_no': permit_no,
                'serial_number': serial_number,
                'completed_date': completed_date,
                'fanciful_name': fanciful_name if fanciful_name else None,
                'brand_name': brand_name,
                'origin_code': origin_code,
                'origin_desc': origin_desc,
                'class_type_code': class_type_code,
                'class_type_desc': class_type_desc,
                'source_url': detail_url,
                'inferred_abv': inferred_abv
            }

            records.append(record)

            # Stop if we've hit the limit
            if len(records) >= limit:
                break

        return records

    def infer_abv_from_class_type(self, class_type_desc: str) -> Optional[float]:
        """
        Infer ABV from TTB class/type description using regulatory ranges.

        TTB class/type descriptions typically don't include ABV, but we can infer
        typical values based on 27 CFR regulations:
        - Wine: 7-14% (table wine), 14-24% (dessert wine)
        - Distilled Spirits: 40-50%
        - Malt Beverages: 3-6% (standard), 7-12% (high-gravity)

        Args:
            class_type_desc: Class/type description (e.g., "TABLE RED WINE")

        Returns:
            Inferred ABV as float, or None if cannot infer
        """
        # Normalize
        normalized = class_type_desc.upper().strip()

        # Direct lookup
        if normalized in self.ABV_RANGES:
            return self.ABV_RANGES[normalized]

        # Fuzzy matching for wine
        if any(kw in normalized for kw in ['TABLE WINE', 'WHITE WINE', 'RED WINE']):
            return 12.0
        if any(kw in normalized for kw in ['DESSERT', 'PORT', 'SHERRY', 'COOKING']):
            return 18.0
        if any(kw in normalized for kw in ['SPARKLING', 'CHAMPAGNE']):
            return 12.0

        # Fuzzy matching for spirits
        if any(kw in normalized for kw in ['WHISKEY', 'WHISKY', 'BOURBON']):
            return 45.0
        if 'GIN' in normalized:
            return 40.0
        if 'VODKA' in normalized:
            return 40.0
        if 'RUM' in normalized:
            return 40.0
        if 'TEQUILA' in normalized:
            return 40.0
        if 'BRANDY' in normalized:
            return 40.0

        # Fuzzy matching for malt beverages
        if any(kw in normalized for kw in ['BEER', 'LAGER', 'ALE']):
            return 5.0
        if 'MALT BEVERAGE' in normalized:
            return 5.0
        if 'IPA' in normalized or 'INDIA PALE ALE' in normalized:
            return 6.5
        if 'STOUT' in normalized or 'PORTER' in normalized:
            return 6.0

        # Default fallback by category hint
        if any(kw in normalized for kw in ['WINE']):
            return 12.0
        if any(kw in normalized for kw in ['SPIRIT', 'LIQUOR', 'LIQUEUR']):
            return 40.0
        if 'MALT' in normalized:
            return 5.0

        return None

    def get_category_from_class_type(self, class_type_desc: str, class_type_code: str) -> str:
        """
        Map TTB class/type to beverage category.

        Args:
            class_type_desc: Class/type description
            class_type_code: Numeric class/type code

        Returns:
            Category: 'wine', 'distilled_spirits', or 'malt_beverage'
        """
        normalized = class_type_desc.upper()

        # Wine indicators
        if any(kw in normalized for kw in [
            'WINE', 'CHAMPAGNE', 'PORT', 'SHERRY', 'DESSERT', 'TABLE'
        ]):
            return 'wine'

        # Spirits indicators
        if any(kw in normalized for kw in [
            'WHISKEY', 'WHISKY', 'BOURBON', 'GIN', 'VODKA', 'RUM',
            'TEQUILA', 'BRANDY', 'LIQUEUR', 'SPIRIT', 'DISTILLED'
        ]):
            return 'distilled_spirits'

        # Malt beverage indicators
        if any(kw in normalized for kw in [
            'BEER', 'ALE', 'LAGER', 'MALT', 'IPA', 'STOUT', 'PORTER'
        ]):
            return 'malt_beverage'

        # Fallback to class code ranges
        try:
            code = int(class_type_code)
            if 80 <= code <= 89:
                return 'wine'
            elif 100 <= code <= 699:
                return 'distilled_spirits'
            elif 900 <= code <= 999:
                return 'malt_beverage'
        except (ValueError, TypeError):
            pass

        # Default to wine
        return 'wine'


def main():
    """Test the TTB COLA client."""
    import argparse

    parser = argparse.ArgumentParser(description='Test TTB COLA client')
    parser.add_argument('--category', choices=['wine', 'distilled_spirits', 'malt_beverage', 'all'],
                        default='all', help='Category to search')
    parser.add_argument('--limit', type=int, default=20, help='Max records to fetch')
    parser.add_argument('--months', type=int, default=6, help='Look back period in months')
    args = parser.parse_args()

    client = TTBCOLAClient()

    categories = ['wine', 'distilled_spirits', 'malt_beverage'] if args.category == 'all' else [args.category]

    for cat in categories:
        print(f"\n{'='*80}")
        print(f"Category: {cat.upper().replace('_', ' ')}")
        print(f"{'='*80}\n")

        records = client.search_recent_approvals(category=cat, limit=args.limit, months_back=args.months)

        print(f"✅ Found {len(records)} records\n")

        for i, record in enumerate(records[:10], 1):
            print(f"{i}. {record['brand_name']}")
            print(f"   Class/Type: {record['class_type_desc']}")
            print(f"   Origin: {record['origin_desc']}")
            print(f"   Inferred ABV: {record['inferred_abv']}%")
            print(f"   TTB ID: {record['ttb_id']}")
            print(f"   Completed: {record['completed_date']}")
            if record['fanciful_name']:
                print(f"   Fanciful Name: {record['fanciful_name']}")
            print()


if __name__ == '__main__':
    main()

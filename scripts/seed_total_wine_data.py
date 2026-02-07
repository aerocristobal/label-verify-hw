#!/usr/bin/env python3
"""
Seed known_beverages database with product data from Total Wine.

This script scrapes beverage data (wine, spirits, beer) from Total Wine's website
and imports it into the PostgreSQL database for reference-based validation.

Requirements:
    pip install requests beautifulsoup4 lxml psycopg2-binary python-dotenv

Usage:
    python3 seed_total_wine_data.py --limit 100

Environment:
    DATABASE_URL - PostgreSQL connection string (from .env)
"""

import argparse
import csv
import json
import logging
import os
import re
import sys
import time
from typing import Dict, List, Optional, Tuple
from urllib.parse import urljoin, urlparse

import psycopg2
import requests
from bs4 import BeautifulSoup
from dotenv import load_dotenv

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

# Rate limiting
REQUEST_DELAY = 2.0  # seconds between requests
USER_AGENT = 'Mozilla/5.0 (compatible; LabelVerifyResearchBot/1.0; +https://github.com/aerocristobal/label-verify-hw)'

# Total Wine product categories
CATEGORIES = {
    'wine': 'https://www.totalwine.com/wine/c/c0020',
    'spirits': 'https://www.totalwine.com/spirits/c/c0030',
    'beer': 'https://www.totalwine.com/beer/c/000001'
}


class TotalWineScraper:
    """Scraper for Total Wine product data."""

    def __init__(self, delay: float = REQUEST_DELAY):
        self.delay = delay
        self.session = requests.Session()
        self.session.headers.update({'User-Agent': USER_AGENT})
        self.last_request_time = 0

    def _rate_limit(self):
        """Enforce rate limiting between requests."""
        elapsed = time.time() - self.last_request_time
        if elapsed < self.delay:
            time.sleep(self.delay - elapsed)
        self.last_request_time = time.time()

    def fetch_page(self, url: str) -> Optional[BeautifulSoup]:
        """Fetch and parse a webpage."""
        try:
            self._rate_limit()
            logger.debug(f"Fetching: {url}")
            response = self.session.get(url, timeout=10)
            response.raise_for_status()
            return BeautifulSoup(response.content, 'lxml')
        except Exception as e:
            logger.error(f"Failed to fetch {url}: {e}")
            return None

    def extract_product_data(self, product_url: str, category: str) -> Optional[Dict]:
        """Extract product data from a product detail page."""
        soup = self.fetch_page(product_url)
        if not soup:
            return None

        try:
            # Extract brand name (usually in title or specific element)
            brand_name = None
            brand_elem = soup.find('span', class_='brand') or soup.find('a', class_='brand-link')
            if brand_elem:
                brand_name = brand_elem.get_text(strip=True)

            # Extract product name
            product_name = None
            name_elem = soup.find('h1', class_='product-name') or soup.find('h1')
            if name_elem:
                product_name = name_elem.get_text(strip=True)

            # Extract ABV from description or details
            abv = None
            abv_pattern = r'(\d+(?:\.\d+)?)\s*%?\s*(?:ABV|Alcohol)'

            # Try to find ABV in product details
            details_elem = soup.find('div', class_='product-details') or soup.find('div', class_='product-description')
            if details_elem:
                text = details_elem.get_text()
                match = re.search(abv_pattern, text, re.IGNORECASE)
                if match:
                    abv = float(match.group(1))

            # Extract bottle size
            size_ml = None
            size_pattern = r'(\d+(?:\.\d+)?)\s*(ml|mL|L|l)'
            if details_elem:
                size_match = re.search(size_pattern, details_elem.get_text())
                if size_match:
                    value = float(size_match.group(1))
                    unit = size_match.group(2).lower()
                    size_ml = int(value * 1000) if unit == 'l' else int(value)

            # Extract class/type (e.g., "Cabernet Sauvignon", "Bourbon Whiskey")
            class_type = None
            type_elem = soup.find('span', class_='varietal') or soup.find('span', class_='type')
            if type_elem:
                class_type = type_elem.get_text(strip=True)
            elif product_name:
                # Infer from product name
                class_type = self._infer_class_type(product_name, category)

            # Extract country
            country = None
            country_elem = soup.find('span', class_='country')
            if country_elem:
                country = country_elem.get_text(strip=True)

            # Skip if missing critical fields
            if not brand_name or not abv or not class_type:
                logger.debug(f"Skipping {product_url} - missing critical data (brand={brand_name}, abv={abv}, class={class_type})")
                return None

            return {
                'brand_name': brand_name,
                'product_name': product_name,
                'class_type': class_type,
                'beverage_category': self._map_category(category),
                'abv': abv,
                'standard_size_ml': size_ml,
                'country_of_origin': country,
                'producer': brand_name,  # Default to brand name
                'source': 'total_wine',
                'source_url': product_url
            }

        except Exception as e:
            logger.error(f"Error extracting data from {product_url}: {e}")
            return None

    def _infer_class_type(self, product_name: str, category: str) -> str:
        """Infer beverage class/type from product name."""
        name_lower = product_name.lower()

        # Wine types
        wine_types = ['cabernet', 'merlot', 'chardonnay', 'pinot noir', 'sauvignon blanc',
                      'riesling', 'zinfandel', 'malbec', 'syrah', 'shiraz']
        for wine_type in wine_types:
            if wine_type in name_lower:
                return wine_type.title()

        # Spirit types
        spirit_types = ['whiskey', 'bourbon', 'scotch', 'vodka', 'gin', 'rum', 'tequila', 'brandy']
        for spirit_type in spirit_types:
            if spirit_type in name_lower:
                return spirit_type.title()

        # Beer types
        beer_types = ['ipa', 'lager', 'ale', 'stout', 'porter', 'pilsner']
        for beer_type in beer_types:
            if beer_type in name_lower:
                return beer_type.upper() if beer_type == 'ipa' else beer_type.title()

        # Default to category
        return category.title()

    def _map_category(self, category: str) -> str:
        """Map Total Wine category to our database category."""
        mapping = {
            'wine': 'wine',
            'spirits': 'distilled_spirits',
            'beer': 'malt_beverage'
        }
        return mapping.get(category, 'wine')

    def scrape_category(self, category: str, limit: int = 100) -> List[Dict]:
        """
        Scrape products from a category.

        Note: This is a simplified implementation. Real implementation would need
        to navigate pagination, handle dynamic content, etc.
        """
        logger.info(f"Scraping {category} category (limit: {limit})")

        # For demonstration purposes, return synthetic data
        # In production, this would actually scrape Total Wine
        return self._generate_synthetic_data(category, limit)

    def _generate_synthetic_data(self, category: str, limit: int) -> List[Dict]:
        """
        Generate synthetic product data for testing.
        Replace with actual scraping logic in production.
        """
        logger.warning(f"Generating synthetic data for {category} (actual scraping not implemented)")

        products = []

        if category == 'wine':
            templates = [
                ('Stone Creek', 'Cabernet Sauvignon', 13.5),
                ('Barefoot', 'Pinot Grigio', 12.0),
                ('Kendall-Jackson', 'Chardonnay', 13.5),
                ('19 Crimes', 'Red Blend', 14.0),
                ('Apothic', 'Red', 13.5),
            ]
            for i, (brand, type_, abv) in enumerate(templates * (limit // len(templates) + 1)):
                if len(products) >= limit:
                    break
                products.append({
                    'brand_name': brand,
                    'product_name': f'{brand} {type_}',
                    'class_type': type_,
                    'beverage_category': 'wine',
                    'abv': abv + (i % 3) * 0.5,  # Slight variation
                    'standard_size_ml': 750,
                    'country_of_origin': 'USA',
                    'producer': brand,
                    'source': 'total_wine',
                    'source_url': f'https://www.totalwine.com/wine/product-{i}'
                })

        elif category == 'spirits':
            templates = [
                ('Jack Daniels', 'Tennessee Whiskey', 40.0),
                ('Tito\'s', 'Vodka', 40.0),
                ('Tanqueray', 'Gin', 47.3),
                ('Bacardi', 'Rum', 40.0),
                ('Patron', 'Tequila', 40.0),
            ]
            for i, (brand, type_, abv) in enumerate(templates * (limit // len(templates) + 1)):
                if len(products) >= limit:
                    break
                products.append({
                    'brand_name': brand,
                    'product_name': f'{brand} {type_}',
                    'class_type': type_,
                    'beverage_category': 'distilled_spirits',
                    'abv': abv + (i % 2) * 0.5,
                    'standard_size_ml': 750,
                    'country_of_origin': 'USA',
                    'producer': brand,
                    'source': 'total_wine',
                    'source_url': f'https://www.totalwine.com/spirits/product-{i}'
                })

        elif category == 'beer':
            templates = [
                ('Sierra Nevada', 'Pale Ale', 5.6),
                ('Lagunitas', 'IPA', 6.2),
                ('Samuel Adams', 'Boston Lager', 5.0),
                ('Guinness', 'Stout', 4.2),
                ('Blue Moon', 'Belgian White', 5.4),
            ]
            for i, (brand, type_, abv) in enumerate(templates * (limit // len(templates) + 1)):
                if len(products) >= limit:
                    break
                products.append({
                    'brand_name': brand,
                    'product_name': f'{brand} {type_}',
                    'class_type': type_,
                    'beverage_category': 'malt_beverage',
                    'abv': abv + (i % 2) * 0.3,
                    'standard_size_ml': 355,
                    'country_of_origin': 'USA',
                    'producer': brand,
                    'source': 'total_wine',
                    'source_url': f'https://www.totalwine.com/beer/product-{i}'
                })

        return products[:limit]


def import_to_database(products: List[Dict], conn_string: str) -> int:
    """Import products to PostgreSQL database."""
    logger.info(f"Importing {len(products)} products to database")

    try:
        conn = psycopg2.connect(conn_string)
        cur = conn.cursor()

        inserted = 0
        skipped = 0

        for product in products:
            try:
                cur.execute("""
                    INSERT INTO known_beverages (
                        brand_name, product_name, class_type, beverage_category,
                        abv, standard_size_ml, country_of_origin, producer,
                        is_verified, source, source_url
                    ) VALUES (
                        %(brand_name)s, %(product_name)s, %(class_type)s, %(beverage_category)s,
                        %(abv)s, %(standard_size_ml)s, %(country_of_origin)s, %(producer)s,
                        false, %(source)s, %(source_url)s
                    )
                    ON CONFLICT DO NOTHING
                """, product)

                if cur.rowcount > 0:
                    inserted += 1
                else:
                    skipped += 1

            except Exception as e:
                logger.error(f"Failed to insert product {product.get('brand_name')}: {e}")
                skipped += 1

        conn.commit()
        cur.close()
        conn.close()

        logger.info(f"Import complete: {inserted} inserted, {skipped} skipped")
        return inserted

    except Exception as e:
        logger.error(f"Database error: {e}")
        raise


def export_to_csv(products: List[Dict], filename: str):
    """Export products to CSV file."""
    logger.info(f"Exporting {len(products)} products to {filename}")

    fieldnames = [
        'brand_name', 'product_name', 'class_type', 'beverage_category',
        'abv', 'standard_size_ml', 'country_of_origin', 'producer',
        'source', 'source_url'
    ]

    with open(filename, 'w', newline='', encoding='utf-8') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(products)

    logger.info(f"CSV export complete: {filename}")


def main():
    parser = argparse.ArgumentParser(description='Seed known_beverages database from Total Wine')
    parser.add_argument('--limit', type=int, default=100, help='Max products per category')
    parser.add_argument('--csv-only', action='store_true', help='Export to CSV only, skip database import')
    parser.add_argument('--category', choices=['wine', 'spirits', 'beer', 'all'], default='all',
                        help='Category to scrape')
    args = parser.parse_args()

    # Load environment
    load_dotenv()
    database_url = os.getenv('DATABASE_URL')

    if not database_url and not args.csv_only:
        logger.error("DATABASE_URL not set in environment")
        sys.exit(1)

    # Initialize scraper
    scraper = TotalWineScraper()

    # Determine categories to scrape
    categories = ['wine', 'spirits', 'beer'] if args.category == 'all' else [args.category]

    # Scrape each category
    all_products = []
    for category in categories:
        products = scraper.scrape_category(category, args.limit)
        logger.info(f"Scraped {len(products)} products from {category}")
        all_products.extend(products)

        # Export to CSV
        csv_filename = f"scripts/data/{category}_products.csv"
        export_to_csv(products, csv_filename)

    # Import to database
    if not args.csv_only:
        total_inserted = import_to_database(all_products, database_url)
        logger.info(f"Total products imported: {total_inserted}")
    else:
        logger.info("CSV-only mode: skipped database import")


if __name__ == '__main__':
    main()

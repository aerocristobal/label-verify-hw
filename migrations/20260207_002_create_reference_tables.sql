-- Create reference tables for database-backed beverage validation

-- 1. known_beverages - Reference data for real beverages
CREATE TABLE IF NOT EXISTS known_beverages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    brand_name VARCHAR(200) NOT NULL,
    product_name VARCHAR(300), -- e.g., "Reserve Cabernet Sauvignon"
    class_type VARCHAR(200) NOT NULL, -- e.g., "Cabernet Sauvignon", "Bourbon Whiskey"
    beverage_category VARCHAR(50) NOT NULL, -- 'wine', 'distilled_spirits', 'malt_beverage'
    abv DECIMAL(4,2) NOT NULL, -- Alcohol by volume (e.g., 13.5)
    standard_size_ml INTEGER, -- Typical bottle size (e.g., 750)
    country_of_origin VARCHAR(100),
    producer VARCHAR(200),
    is_verified BOOLEAN DEFAULT false, -- Manually verified by admin
    source VARCHAR(100), -- 'total_wine', 'manual', 'ttb_cola'
    source_url TEXT, -- Original product URL
    notes TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT abv_valid_range CHECK (abv >= 0 AND abv <= 100)
);

-- Create indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_known_beverages_brand ON known_beverages(LOWER(brand_name));
CREATE INDEX IF NOT EXISTS idx_known_beverages_class ON known_beverages(LOWER(class_type));
CREATE INDEX IF NOT EXISTS idx_known_beverages_category ON known_beverages(beverage_category);
CREATE INDEX IF NOT EXISTS idx_known_beverages_abv ON known_beverages(abv);

-- Create unique constraint to prevent duplicates
CREATE UNIQUE INDEX IF NOT EXISTS idx_known_beverages_unique
    ON known_beverages(LOWER(brand_name), LOWER(COALESCE(product_name, '')), abv);

-- Add trigger for auto-updating updated_at
CREATE TRIGGER update_known_beverages_updated_at
    BEFORE UPDATE ON known_beverages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Add comments
COMMENT ON TABLE known_beverages IS 'Reference database of known beverage products for cross-validation';
COMMENT ON COLUMN known_beverages.is_verified IS 'True if manually verified by admin, false for auto-imported data';
COMMENT ON COLUMN known_beverages.source IS 'Data source: total_wine, manual, ttb_cola';


-- 2. beverage_category_rules - ABV ranges and validation rules per category
CREATE TABLE IF NOT EXISTS beverage_category_rules (
    id SERIAL PRIMARY KEY,
    category VARCHAR(50) NOT NULL UNIQUE, -- 'wine', 'distilled_spirits', 'malt_beverage'
    min_abv DECIMAL(4,2) NOT NULL,
    max_abv DECIMAL(4,2) NOT NULL,
    typical_min_abv DECIMAL(4,2), -- Typical range (narrower)
    typical_max_abv DECIMAL(4,2),
    cfr_reference VARCHAR(100), -- e.g., "27 CFR Part 4"
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT category_abv_ranges_valid CHECK (
        min_abv >= 0 AND
        max_abv <= 100 AND
        min_abv < max_abv AND
        (typical_min_abv IS NULL OR typical_min_abv >= min_abv) AND
        (typical_max_abv IS NULL OR typical_max_abv <= max_abv) AND
        (typical_min_abv IS NULL OR typical_max_abv IS NULL OR typical_min_abv < typical_max_abv)
    )
);

-- Seed category rules with TTB-compliant data
INSERT INTO beverage_category_rules (category, min_abv, max_abv, typical_min_abv, typical_max_abv, cfr_reference, description) VALUES
('wine', 5.0, 24.0, 8.0, 15.0, '27 CFR Part 4', 'Table wines typically 8-15% ABV, dessert wines up to 24%'),
('distilled_spirits', 30.0, 95.0, 40.0, 50.0, '27 CFR Part 5', 'Most spirits are 40-50% ABV (80-100 proof)'),
('malt_beverage', 0.5, 15.0, 3.0, 8.0, '27 CFR Part 7', 'Most beers 3-8% ABV, barleywines/imperials up to 15%')
ON CONFLICT (category) DO NOTHING;

COMMENT ON TABLE beverage_category_rules IS 'TTB-compliant ABV ranges and validation rules for beverage categories';
COMMENT ON COLUMN beverage_category_rules.cfr_reference IS 'Code of Federal Regulations reference';


-- 3. beverage_match_history - Track database matches for analytics
CREATE TABLE IF NOT EXISTS beverage_match_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id UUID NOT NULL REFERENCES verification_jobs(id) ON DELETE CASCADE,
    matched_beverage_id UUID REFERENCES known_beverages(id) ON DELETE SET NULL,
    match_type VARCHAR(50) NOT NULL, -- 'exact', 'fuzzy', 'category_only', 'no_match'
    match_confidence DECIMAL(4,3), -- 0.000-1.000
    abv_deviation DECIMAL(5,2), -- Difference from expected ABV
    created_at TIMESTAMPTZ DEFAULT NOW(),

    CONSTRAINT match_confidence_valid CHECK (match_confidence IS NULL OR (match_confidence >= 0 AND match_confidence <= 1)),
    CONSTRAINT match_type_valid CHECK (match_type IN ('exact', 'fuzzy', 'category_only', 'no_match'))
);

-- Create indexes for analytics queries
CREATE INDEX IF NOT EXISTS idx_match_history_job ON beverage_match_history(job_id);
CREATE INDEX IF NOT EXISTS idx_match_history_beverage ON beverage_match_history(matched_beverage_id) WHERE matched_beverage_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_match_history_type ON beverage_match_history(match_type);
CREATE INDEX IF NOT EXISTS idx_match_history_created_at ON beverage_match_history(created_at DESC);

COMMENT ON TABLE beverage_match_history IS 'Tracks database match results for verification jobs (analytics and debugging)';
COMMENT ON COLUMN beverage_match_history.abv_deviation IS 'Absolute difference between extracted ABV and database/category ABV';

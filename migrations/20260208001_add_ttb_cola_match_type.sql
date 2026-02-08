-- Add 'ttb_cola_lookup' to the match_type CHECK constraint
-- Required for the TTB COLA read-through cache feature

ALTER TABLE beverage_match_history DROP CONSTRAINT IF EXISTS match_type_valid;
ALTER TABLE beverage_match_history ADD CONSTRAINT match_type_valid
    CHECK (match_type IN ('exact', 'fuzzy', 'category_only', 'no_match', 'ttb_cola_lookup'));

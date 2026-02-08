use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::beverage::{BeverageCategoryRule, KnownBeverage, NewMatchHistory};

/// Find known beverages by brand and class/type (case-insensitive)
pub async fn find_known_beverage(
    pool: &PgPool,
    brand: &str,
    class_type: &str,
) -> Result<Vec<KnownBeverage>, sqlx::Error> {
    sqlx::query_as!(
        KnownBeverage,
        r#"
        SELECT id, brand_name as "brand_name!", product_name, class_type as "class_type!", beverage_category as "beverage_category!",
               abv::float8 as "abv!", standard_size_ml, country_of_origin, producer,
               is_verified as "is_verified!", source as "source!", source_url, notes, created_at as "created_at!", updated_at as "updated_at!"
        FROM known_beverages
        WHERE LOWER(brand_name) = LOWER($1)
          AND LOWER(class_type) = LOWER($2)
        ORDER BY is_verified DESC, abv ASC
        LIMIT 10
        "#,
        brand,
        class_type
    )
    .fetch_all(pool)
    .await
}

/// Find known beverages by brand only (for fuzzy class matching)
pub async fn find_known_beverage_by_brand(
    pool: &PgPool,
    brand: &str,
) -> Result<Vec<KnownBeverage>, sqlx::Error> {
    sqlx::query_as!(
        KnownBeverage,
        r#"
        SELECT id, brand_name as "brand_name!", product_name, class_type as "class_type!", beverage_category as "beverage_category!",
               abv::float8 as "abv!", standard_size_ml, country_of_origin, producer,
               is_verified as "is_verified!", source as "source!", source_url, notes, created_at as "created_at!", updated_at as "updated_at!"
        FROM known_beverages
        WHERE LOWER(brand_name) = LOWER($1)
        ORDER BY is_verified DESC
        LIMIT 10
        "#,
        brand
    )
    .fetch_all(pool)
    .await
}

/// Check if a cache entry is stale (older than threshold)
///
/// Default threshold: 30 days
///
/// Returns true if the entry is older than the threshold and should be refreshed.
pub fn is_cache_stale(created_at: DateTime<Utc>, threshold_days: i64) -> bool {
    let now = Utc::now();
    let age = now.signed_duration_since(created_at);
    age.num_days() > threshold_days
}

/// Find known beverage with cache freshness information
///
/// Returns tuple of (beverage, is_stale) where is_stale indicates if the cache entry
/// is older than 30 days and should be refreshed.
pub async fn find_known_beverage_with_staleness(
    pool: &PgPool,
    brand: &str,
    class_type: &str,
    staleness_threshold_days: i64,
) -> Result<Option<(KnownBeverage, bool)>, sqlx::Error> {
    let beverages = find_known_beverage(pool, brand, class_type).await?;

    if let Some(beverage) = beverages.first() {
        let is_stale = is_cache_stale(beverage.created_at, staleness_threshold_days);
        Ok(Some((beverage.clone(), is_stale)))
    } else {
        Ok(None)
    }
}

/// Get category rule for a beverage class/type
/// Maps class_type to category (wine/distilled_spirits/malt_beverage)
pub async fn get_category_rule(
    pool: &PgPool,
    class_type: &str,
) -> Result<Option<BeverageCategoryRule>, sqlx::Error> {
    // Determine category from class_type
    let category = infer_category_from_class(class_type);

    sqlx::query_as!(
        BeverageCategoryRule,
        r#"
        SELECT id, category as "category!", min_abv::float8 as "min_abv!", max_abv::float8 as "max_abv!",
               typical_min_abv::float8 as "typical_min_abv", typical_max_abv::float8 as "typical_max_abv",
               cfr_reference, description, created_at as "created_at!"
        FROM beverage_category_rules
        WHERE category = $1
        "#,
        category
    )
    .fetch_optional(pool)
    .await
}

/// Infer beverage category from class/type string
fn infer_category_from_class(class_type: &str) -> String {
    let lower = class_type.to_lowercase();

    // Wine types
    if lower.contains("wine")
        || lower.contains("cabernet")
        || lower.contains("merlot")
        || lower.contains("chardonnay")
        || lower.contains("pinot")
        || lower.contains("sauvignon")
        || lower.contains("riesling")
        || lower.contains("zinfandel")
        || lower.contains("malbec")
        || lower.contains("syrah")
        || lower.contains("shiraz")
        || lower.contains("prosecco")
        || lower.contains("champagne")
    {
        return "wine".to_string();
    }

    // Distilled spirits types
    if lower.contains("whiskey")
        || lower.contains("whisky")
        || lower.contains("bourbon")
        || lower.contains("scotch")
        || lower.contains("vodka")
        || lower.contains("gin")
        || lower.contains("rum")
        || lower.contains("tequila")
        || lower.contains("brandy")
        || lower.contains("cognac")
        || lower.contains("liqueur")
    {
        return "distilled_spirits".to_string();
    }

    // Malt beverage types
    if lower.contains("beer")
        || lower.contains("ale")
        || lower.contains("lager")
        || lower.contains("ipa")
        || lower.contains("stout")
        || lower.contains("porter")
        || lower.contains("pilsner")
        || lower.contains("malt")
    {
        return "malt_beverage".to_string();
    }

    // Default to wine (most common)
    "wine".to_string()
}

/// Record match history for analytics
pub async fn record_match_history(
    pool: &PgPool,
    match_history: NewMatchHistory,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO beverage_match_history
            (id, job_id, matched_beverage_id, match_type, match_confidence, abv_deviation)
        VALUES ($1, $2, $3, $4, $5::float8, $6::float8)
        "#,
        id,
        match_history.job_id,
        match_history.matched_beverage_id,
        match_history.match_type,
        match_history.match_confidence,
        match_history.abv_deviation
    )
    .execute(pool)
    .await?;

    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_category_wine() {
        assert_eq!(infer_category_from_class("Cabernet Sauvignon"), "wine");
        assert_eq!(infer_category_from_class("Red Wine"), "wine");
        assert_eq!(infer_category_from_class("Chardonnay"), "wine");
    }

    #[test]
    fn test_infer_category_spirits() {
        assert_eq!(infer_category_from_class("Bourbon Whiskey"), "distilled_spirits");
        assert_eq!(infer_category_from_class("Vodka"), "distilled_spirits");
        assert_eq!(infer_category_from_class("Tequila"), "distilled_spirits");
    }

    #[test]
    fn test_infer_category_beer() {
        assert_eq!(infer_category_from_class("IPA"), "malt_beverage");
        assert_eq!(infer_category_from_class("Lager Beer"), "malt_beverage");
        assert_eq!(infer_category_from_class("Stout"), "malt_beverage");
    }

    #[test]
    fn test_is_cache_stale() {
        use chrono::{Duration, Utc};

        let now = Utc::now();

        // Fresh entry (1 day old)
        let fresh = now - Duration::days(1);
        assert!(!is_cache_stale(fresh, 30), "1-day-old entry should not be stale");

        // Borderline entry (30 days old)
        let borderline = now - Duration::days(30);
        assert!(!is_cache_stale(borderline, 30), "30-day-old entry should not be stale");

        // Stale entry (31 days old)
        let stale = now - Duration::days(31);
        assert!(is_cache_stale(stale, 30), "31-day-old entry should be stale");

        // Very stale entry (90 days old)
        let very_stale = now - Duration::days(90);
        assert!(is_cache_stale(very_stale, 30), "90-day-old entry should be stale");

        // Custom threshold (7 days)
        let week_old = now - Duration::days(8);
        assert!(is_cache_stale(week_old, 7), "8-day-old entry should be stale with 7-day threshold");
    }
}

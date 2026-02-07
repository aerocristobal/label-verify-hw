use serde::{Deserialize, Serialize};
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

/// Known beverage from reference database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct KnownBeverage {
    pub id: Uuid,
    pub brand_name: String,
    pub product_name: Option<String>,
    pub class_type: String,
    pub beverage_category: String,
    pub abv: f64,
    pub standard_size_ml: Option<i32>,
    pub country_of_origin: Option<String>,
    pub producer: Option<String>,
    pub is_verified: bool,
    pub source: String,
    pub source_url: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// TTB-compliant ABV ranges for beverage categories
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BeverageCategoryRule {
    pub id: i32,
    pub category: String,
    pub min_abv: f64,
    pub max_abv: f64,
    pub typical_min_abv: Option<f64>,
    pub typical_max_abv: Option<f64>,
    pub cfr_reference: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Record of database match for a verification job
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BeverageMatchHistory {
    pub id: Uuid,
    pub job_id: Uuid,
    pub matched_beverage_id: Option<Uuid>,
    pub match_type: String,
    pub match_confidence: Option<f64>,
    pub abv_deviation: Option<f64>,
    pub created_at: DateTime<Utc>,
}

/// Helper for inserting match history records
#[derive(Debug, Clone)]
pub struct NewMatchHistory {
    pub job_id: Uuid,
    pub matched_beverage_id: Option<Uuid>,
    pub match_type: String,
    pub match_confidence: Option<f64>,
    pub abv_deviation: Option<f64>,
}

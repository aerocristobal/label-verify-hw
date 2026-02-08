use sqlx::PgPool;
use strsim::jaro_winkler;

use crate::db::beverage_queries;
use crate::models::label::{ExtractedLabelFields, FieldVerification, VerificationResult};
use crate::services::ttb_standards;

/// Threshold for fuzzy string matching (0.0 - 1.0).
const MATCH_THRESHOLD: f64 = 0.85;

/// TTB-mandated ABV tolerance: ±0.3 percentage points.
const ABV_TOLERANCE: f64 = 0.3;

/// Cache staleness threshold in days (30 days).
const CACHE_STALENESS_THRESHOLD_DAYS: i64 = 30;

/// Validate extracted label fields against expected values and TTB rules.
///
/// Performs:
/// - Brand name fuzzy matching
/// - Class/type validation against TTB standards of identity
/// - ABV tolerance checking (±0.3% per 27 CFR)
/// - Net contents format validation
/// - Same field-of-vision checks (brand, class/type, ABV must appear together)
/// - Mandatory field presence verification
pub fn verify_label(
    extracted: &ExtractedLabelFields,
    expected_brand: Option<&str>,
    expected_class: Option<&str>,
    expected_abv: Option<f64>,
) -> VerificationResult {
    let mut field_results = Vec::new();

    // ── Brand Name Verification (fuzzy match) ────────────────────────
    if let Some(expected) = expected_brand {
        let score = jaro_winkler(&extracted.brand_name.to_lowercase(), &expected.to_lowercase());
        field_results.push(FieldVerification {
            field_name: "brand_name".to_string(),
            expected: Some(expected.to_string()),
            extracted: extracted.brand_name.clone(),
            matches: score >= MATCH_THRESHOLD,
            similarity_score: score,
        });
    }

    // ── Class/Type Verification ──────────────────────────────────────
    // First: fuzzy match against expected value (if provided)
    if let Some(expected) = expected_class {
        let score = jaro_winkler(&extracted.class_type.to_lowercase(), &expected.to_lowercase());
        field_results.push(FieldVerification {
            field_name: "class_type".to_string(),
            expected: Some(expected.to_string()),
            extracted: extracted.class_type.clone(),
            matches: score >= MATCH_THRESHOLD,
            similarity_score: score,
        });
    }

    // Second: validate against TTB standards of identity (27 CFR)
    if !extracted.class_type.is_empty() {
        let classification = ttb_standards::validate_classification(&extracted.class_type);

        field_results.push(FieldVerification {
            field_name: "class_type_ttb_valid".to_string(),
            expected: classification
                .matched_standard
                .clone()
                .or(Some("Valid TTB designation".to_string())),
            extracted: extracted.class_type.clone(),
            matches: classification.is_valid,
            similarity_score: classification.similarity,
        });

        // Flag if spelling correction detected
        if let Some(ref correction) = classification.spelling_correction {
            field_results.push(FieldVerification {
                field_name: "class_type_spelling".to_string(),
                expected: Some(correction.clone()),
                extracted: extracted.class_type.clone(),
                matches: false, // Misspelling is a mismatch
                similarity_score: classification.similarity,
            });
        }

        // Flag if fanciful name requires statement of composition
        if classification.requires_composition_statement {
            field_results.push(FieldVerification {
                field_name: "composition_statement_required".to_string(),
                expected: Some("Statement of composition required for fanciful names".to_string()),
                extracted: extracted.class_type.clone(),
                matches: false, // Flagged for review
                similarity_score: 0.0,
            });
        }
    }

    // ── ABV Verification (±0.3% tolerance per 27 CFR) ────────────────
    if let Some(expected) = expected_abv {
        let diff = (extracted.abv - expected).abs();
        let within_tolerance = diff <= ABV_TOLERANCE;
        let score = if within_tolerance {
            1.0
        } else {
            (1.0 - (diff / 100.0)).max(0.0)
        };
        field_results.push(FieldVerification {
            field_name: "abv".to_string(),
            expected: Some(format!("{:.1}%", expected)),
            extracted: format!("{:.1}%", extracted.abv),
            matches: within_tolerance,
            similarity_score: score,
        });
    }

    // ── Net Contents Format Validation ───────────────────────────────
    if !extracted.net_contents.is_empty() {
        let (is_valid, value_ml, unit) =
            ttb_standards::validate_net_contents(&extracted.net_contents);
        let detail = match (value_ml, &unit) {
            (Some(v), Some(u)) => format!("{:.0} mL (parsed as {} {})", v, extracted.net_contents, u),
            _ => "Could not parse".to_string(),
        };
        field_results.push(FieldVerification {
            field_name: "net_contents_format".to_string(),
            expected: Some("Valid volume with unit (mL or L)".to_string()),
            extracted: detail,
            matches: is_valid,
            similarity_score: if is_valid { 1.0 } else { 0.0 },
        });
    }

    // ── Mandatory Field Presence (27 CFR) ────────────────────────────
    // Brand name must be present
    if extracted.brand_name.is_empty() {
        field_results.push(FieldVerification {
            field_name: "brand_name_present".to_string(),
            expected: Some("Required".to_string()),
            extracted: String::new(),
            matches: false,
            similarity_score: 0.0,
        });
    }

    // Class/type must be present
    if extracted.class_type.is_empty() {
        field_results.push(FieldVerification {
            field_name: "class_type_present".to_string(),
            expected: Some("Required".to_string()),
            extracted: String::new(),
            matches: false,
            similarity_score: 0.0,
        });
    }

    // ABV must be present (for beverages ≥0.5% ABV)
    if extracted.abv <= 0.0 {
        field_results.push(FieldVerification {
            field_name: "abv_present".to_string(),
            expected: Some("Required (> 0%)".to_string()),
            extracted: format!("{:.1}%", extracted.abv),
            matches: false,
            similarity_score: 0.0,
        });
    }

    // Net contents must be present
    if extracted.net_contents.is_empty() {
        field_results.push(FieldVerification {
            field_name: "net_contents_present".to_string(),
            expected: Some("Required".to_string()),
            extracted: String::new(),
            matches: false,
            similarity_score: 0.0,
        });
    }

    // ── Same Field of Vision Check (27 CFR 5.63) ─────────────────────
    // Brand name, class/type, and ABV must all appear on the primary label.
    // Since OCR extracts from a single image, we verify all three are present.
    let has_brand = !extracted.brand_name.is_empty();
    let has_class = !extracted.class_type.is_empty();
    let has_abv = extracted.abv > 0.0;
    let same_fov = has_brand && has_class && has_abv;

    field_results.push(FieldVerification {
        field_name: "same_field_of_vision".to_string(),
        expected: Some("Brand, class/type, and ABV in same view".to_string()),
        extracted: format!(
            "brand={}, class={}, abv={}",
            if has_brand { "yes" } else { "no" },
            if has_class { "yes" } else { "no" },
            if has_abv { "yes" } else { "no" },
        ),
        matches: same_fov,
        similarity_score: if same_fov { 1.0 } else { 0.0 },
    });

    // ── Compute Overall Result ───────────────────────────────────────
    let passed = field_results.iter().all(|f| f.matches);
    let confidence_score = if field_results.is_empty() {
        0.0
    } else {
        field_results.iter().map(|f| f.similarity_score).sum::<f64>() / field_results.len() as f64
    };

    VerificationResult {
        passed,
        field_results,
        confidence_score,
        matched_beverage_id: None,
        match_type: "no_match".to_string(),
        match_confidence: 0.0,
        abv_deviation: None,
        category_rule_applied: None,
        warnings: Vec::new(),
    }
}

/// Enhanced validation with database-backed beverage reference checking.
///
/// This async version performs:
/// 1. Exact database lookup by brand + class/type
/// 2. ABV consistency check against known products
/// 3. Category ABV range validation (wine: 5-24%, spirits: 30-95%, beer: 0.5-15%)
/// 4. Fuzzy matching fallback (same as verify_label)
/// 5. Recording of match history for analytics
pub async fn verify_label_with_database(
    pool: &PgPool,
    extracted: &ExtractedLabelFields,
    expected_brand: Option<&str>,
    expected_class: Option<&str>,
    expected_abv: Option<f64>,
) -> Result<VerificationResult, sqlx::Error> {
    // Start with base validation (non-database checks)
    let mut result = verify_label(extracted, expected_brand, expected_class, expected_abv);

    // ── Database Exact Match Lookup with Staleness Check ─────────────
    let db_match_with_staleness = if !extracted.brand_name.is_empty() && !extracted.class_type.is_empty() {
        beverage_queries::find_known_beverage_with_staleness(
            pool,
            &extracted.brand_name,
            &extracted.class_type,
            CACHE_STALENESS_THRESHOLD_DAYS,
        )
        .await?
    } else {
        None
    };

    if let Some((db_match, is_stale)) = db_match_with_staleness {
        // Found exact match in database
        result.matched_beverage_id = Some(db_match.id);
        result.match_type = "exact".to_string();
        result.match_confidence = 1.0;

        // Warn if cache is stale
        if is_stale {
            result.warnings.push(format!(
                "Database cache entry is older than {} days. Consider refreshing TTB COLA data for brand '{}' (source: {}).",
                CACHE_STALENESS_THRESHOLD_DAYS,
                db_match.brand_name,
                db_match.source
            ));
        }

        // Check ABV consistency
        let abv_diff = (extracted.abv - db_match.abv).abs();
        result.abv_deviation = Some(abv_diff);

        if abv_diff > 1.0 {
            // Flag: ABV differs by >1% from known product
            result.field_results.push(FieldVerification {
                field_name: "abv_database_match".to_string(),
                expected: Some(format!("{:.1}%", db_match.abv)),
                extracted: format!("{:.1}%", extracted.abv),
                matches: false,
                similarity_score: (1.0 - (abv_diff / 100.0)).max(0.0),
            });
            result.passed = false;
        } else {
            // ABV is consistent with database
            result.field_results.push(FieldVerification {
                field_name: "abv_database_match".to_string(),
                expected: Some(format!("{:.1}%", db_match.abv)),
                extracted: format!("{:.1}%", extracted.abv),
                matches: true,
                similarity_score: 1.0 - (abv_diff / 100.0),
            });
        }
    } else {
        // No exact match - try fuzzy brand-only match
        let brand_matches = if !extracted.brand_name.is_empty() {
            beverage_queries::find_known_beverage_by_brand(pool, &extracted.brand_name).await?
        } else {
            Vec::new()
        };

        if let Some(fuzzy_match) = brand_matches.first() {
            // Found brand but not exact class/type
            result.matched_beverage_id = Some(fuzzy_match.id);
            result.match_type = "fuzzy".to_string();

            // Calculate fuzzy match confidence based on class/type similarity
            let class_similarity = jaro_winkler(
                &extracted.class_type.to_lowercase(),
                &fuzzy_match.class_type.to_lowercase(),
            );
            result.match_confidence = class_similarity;

            // Check ABV consistency
            let abv_diff = (extracted.abv - fuzzy_match.abv).abs();
            result.abv_deviation = Some(abv_diff);

            if abv_diff > 2.0 {
                // More lenient threshold for fuzzy matches
                result.field_results.push(FieldVerification {
                    field_name: "abv_database_fuzzy_match".to_string(),
                    expected: Some(format!(
                        "{:.1}% (from similar product: {})",
                        fuzzy_match.abv, fuzzy_match.class_type
                    )),
                    extracted: format!("{:.1}%", extracted.abv),
                    matches: false,
                    similarity_score: (1.0 - (abv_diff / 100.0)).max(0.0),
                });
                result.passed = false;
            }
        }
    }

    // ── Category ABV Range Validation ────────────────────────────────
    if let Some(category_rule) =
        beverage_queries::get_category_rule(pool, &extracted.class_type).await?
    {
        result.category_rule_applied = Some(format!(
            "{} ({:.1}-{:.1}% ABV)",
            category_rule.category, category_rule.min_abv, category_rule.max_abv
        ));

        // Check if ABV is within valid range
        if extracted.abv < category_rule.min_abv || extracted.abv > category_rule.max_abv {
            result.field_results.push(FieldVerification {
                field_name: "abv_category_range".to_string(),
                expected: Some(format!(
                    "{:.1}-{:.1}% ({}, per {})",
                    category_rule.min_abv,
                    category_rule.max_abv,
                    category_rule.category,
                    category_rule
                        .cfr_reference
                        .as_deref()
                        .unwrap_or("27 CFR")
                )),
                extracted: format!("{:.1}%", extracted.abv),
                matches: false,
                similarity_score: 0.0,
            });
            result.passed = false;

            // If no match type yet, set to category_only
            if result.match_type == "no_match" {
                result.match_type = "category_only".to_string();
            }
        } else {
            // Check if within typical range (informational)
            if let (Some(typical_min), Some(typical_max)) =
                (category_rule.typical_min_abv, category_rule.typical_max_abv)
            {
                if extracted.abv < typical_min || extracted.abv > typical_max {
                    result.field_results.push(FieldVerification {
                        field_name: "abv_category_typical_range".to_string(),
                        expected: Some(format!(
                            "{:.1}-{:.1}% (typical for {})",
                            typical_min, typical_max, category_rule.category
                        )),
                        extracted: format!("{:.1}% (unusual but valid)", extracted.abv),
                        matches: true, // Valid but flagged as unusual
                        similarity_score: 0.7,
                    });
                }
            }
        }
    }

    // ── Overall Logical Consistency Check ────────────────────────────
    // Flag if major inconsistencies detected
    let has_major_inconsistency = result
        .field_results
        .iter()
        .any(|f| !f.matches && (f.field_name.contains("abv_database") || f.field_name.contains("abv_category")));

    if has_major_inconsistency {
        result.field_results.push(FieldVerification {
            field_name: "logical_consistency".to_string(),
            expected: Some(format!(
                "{} with appropriate ABV for category",
                extracted.class_type
            )),
            extracted: format!("{} with {:.1}% ABV (inconsistent)", extracted.class_type, extracted.abv),
            matches: false,
            similarity_score: 0.0,
        });
    }

    // Recalculate confidence score with new field results
    result.confidence_score = if result.field_results.is_empty() {
        0.0
    } else {
        result.field_results.iter().map(|f| f.similarity_score).sum::<f64>()
            / result.field_results.len() as f64
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_fields() -> ExtractedLabelFields {
        ExtractedLabelFields {
            brand_name: "Stone Creek Vineyards".to_string(),
            class_type: "Cabernet Sauvignon".to_string(),
            abv: 13.5,
            net_contents: "750 mL".to_string(),
            country_of_origin: Some("USA".to_string()),
            government_warning: Some("GOVERNMENT WARNING: ...".to_string()),
        }
    }

    #[test]
    fn test_exact_match() {
        let fields = sample_fields();
        let result = verify_label(
            &fields,
            Some("Stone Creek Vineyards"),
            Some("Cabernet Sauvignon"),
            Some(13.5),
        );
        // Brand, class, ABV should all match
        let brand = result.field_results.iter().find(|f| f.field_name == "brand_name").unwrap();
        assert!(brand.matches);
        let abv = result.field_results.iter().find(|f| f.field_name == "abv").unwrap();
        assert!(abv.matches);
    }

    #[test]
    fn test_abv_within_tolerance() {
        let fields = sample_fields();
        let result = verify_label(&fields, None, None, Some(13.7)); // 0.2% diff
        let abv = result.field_results.iter().find(|f| f.field_name == "abv").unwrap();
        assert!(abv.matches); // Within ±0.3%
    }

    #[test]
    fn test_abv_outside_tolerance() {
        let fields = sample_fields();
        let result = verify_label(&fields, None, None, Some(14.0)); // 0.5% diff
        let abv = result.field_results.iter().find(|f| f.field_name == "abv").unwrap();
        assert!(!abv.matches); // Outside ±0.3%
    }

    #[test]
    fn test_same_field_of_vision() {
        let fields = sample_fields();
        let result = verify_label(&fields, None, None, None);
        let fov = result.field_results.iter().find(|f| f.field_name == "same_field_of_vision").unwrap();
        assert!(fov.matches);
    }

    #[test]
    fn test_missing_brand_fails_fov() {
        let mut fields = sample_fields();
        fields.brand_name = String::new();
        let result = verify_label(&fields, None, None, None);
        let fov = result.field_results.iter().find(|f| f.field_name == "same_field_of_vision").unwrap();
        assert!(!fov.matches);
    }

    #[test]
    fn test_ttb_classification_check() {
        let fields = sample_fields();
        let result = verify_label(&fields, None, None, None);
        let ttb = result.field_results.iter().find(|f| f.field_name == "class_type_ttb_valid").unwrap();
        assert!(ttb.matches); // "Cabernet Sauvignon" is a valid wine type
    }

    #[test]
    fn test_net_contents_validated() {
        let fields = sample_fields();
        let result = verify_label(&fields, None, None, None);
        let nc = result.field_results.iter().find(|f| f.field_name == "net_contents_format").unwrap();
        assert!(nc.matches);
    }
}

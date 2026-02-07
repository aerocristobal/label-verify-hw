use strsim::jaro_winkler;

use crate::models::label::{ExtractedLabelFields, FieldVerification, VerificationResult};
use crate::services::ttb_standards;

/// Threshold for fuzzy string matching (0.0 - 1.0).
const MATCH_THRESHOLD: f64 = 0.85;

/// TTB-mandated ABV tolerance: ±0.3 percentage points.
const ABV_TOLERANCE: f64 = 0.3;

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
    }
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

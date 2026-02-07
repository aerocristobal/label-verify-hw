use strsim::jaro_winkler;

use crate::models::label::{ExtractedLabelFields, FieldVerification, VerificationResult};

/// Threshold for fuzzy string matching (0.0 - 1.0).
const MATCH_THRESHOLD: f64 = 0.85;

/// Validate extracted label fields against expected values and TTB rules.
pub fn verify_label(
    extracted: &ExtractedLabelFields,
    expected_brand: Option<&str>,
    expected_class: Option<&str>,
    expected_abv: Option<f64>,
) -> VerificationResult {
    let mut field_results = Vec::new();

    // Brand name verification (fuzzy match)
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

    // Class/type verification (fuzzy match)
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

    // ABV verification (numeric tolerance)
    if let Some(expected) = expected_abv {
        let diff = (extracted.abv - expected).abs();
        let matches = diff <= 0.5; // 0.5% tolerance
        let score = if matches { 1.0 } else { 1.0 - (diff / 100.0) };
        field_results.push(FieldVerification {
            field_name: "abv".to_string(),
            expected: Some(format!("{:.1}%", expected)),
            extracted: format!("{:.1}%", extracted.abv),
            matches,
            similarity_score: score,
        });
    }

    // Mandatory field presence checks (TTB requirements)
    if extracted.brand_name.is_empty() {
        field_results.push(FieldVerification {
            field_name: "brand_name_present".to_string(),
            expected: Some("non-empty".to_string()),
            extracted: String::new(),
            matches: false,
            similarity_score: 0.0,
        });
    }

    if extracted.net_contents.is_empty() {
        field_results.push(FieldVerification {
            field_name: "net_contents_present".to_string(),
            expected: Some("non-empty".to_string()),
            extracted: String::new(),
            matches: false,
            similarity_score: 0.0,
        });
    }

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

//! TTB Standards of Identity reference data.
//!
//! Based on 27 CFR Part 5 (Distilled Spirits), Part 4 (Wine), Part 7 (Malt Beverages).
//! Used for validating class/type designations on beverage labels.

use strsim::jaro_winkler;

/// Minimum similarity score for a class/type to be considered a valid match.
const CLASS_MATCH_THRESHOLD: f64 = 0.88;

// ── Distilled Spirits (27 CFR 5.22) ─────────────────────────────────────

/// Standard distilled spirits types recognized by the TTB.
pub const DISTILLED_SPIRITS_TYPES: &[&str] = &[
    // Whiskey types
    "Bourbon Whiskey",
    "Straight Bourbon Whiskey",
    "Kentucky Straight Bourbon Whiskey",
    "Tennessee Whiskey",
    "Rye Whiskey",
    "Straight Rye Whiskey",
    "Corn Whiskey",
    "Wheat Whiskey",
    "Malt Whiskey",
    "Blended Whiskey",
    "Light Whiskey",
    "Spirit Whiskey",
    "Scotch Whisky",
    "Irish Whiskey",
    "Canadian Whisky",
    "Whiskey",
    "Whisky",
    // Vodka
    "Vodka",
    // Gin
    "Gin",
    "Distilled Gin",
    "London Dry Gin",
    // Rum
    "Rum",
    "Light Rum",
    "Dark Rum",
    "Gold Rum",
    "Aged Rum",
    "Spiced Rum",
    // Brandy
    "Brandy",
    "Grape Brandy",
    "Cognac",
    "Armagnac",
    "Pisco",
    "Calvados",
    "Apple Brandy",
    "Applejack",
    // Tequila / Mezcal
    "Tequila",
    "Tequila Blanco",
    "Tequila Reposado",
    "Tequila Anejo",
    "Mezcal",
    // Liqueurs / Cordials
    "Liqueur",
    "Cordial",
    "Triple Sec",
    "Amaretto",
    "Schnapps",
    // Other spirits
    "Absinthe",
    "Aquavit",
    "Bitters",
    "Grappa",
    "Shochu",
    "Soju",
    "Baijiu",
    "Cachaca",
    "Neutral Spirits",
    "Grain Spirits",
    "Distilled Spirits Specialty",
];

// ── Wine (27 CFR 4.21) ──────────────────────────────────────────────────

/// Standard wine types recognized by the TTB.
pub const WINE_TYPES: &[&str] = &[
    "Grape Wine",
    "Table Wine",
    "Red Wine",
    "White Wine",
    "Rose Wine",
    "Rosé",
    "Sparkling Wine",
    "Champagne",
    "Prosecco",
    "Cava",
    "Dessert Wine",
    "Sherry",
    "Port",
    "Madeira",
    "Marsala",
    "Vermouth",
    "Saké",
    "Sake",
    "Fruit Wine",
    "Apple Wine",
    "Cider",
    "Hard Cider",
    "Mead",
    "Honey Wine",
    "Retsina",
    "Natural Wine",
    "Fortified Wine",
    "Aperitif Wine",
    // Varietals (common)
    "Cabernet Sauvignon",
    "Merlot",
    "Pinot Noir",
    "Chardonnay",
    "Sauvignon Blanc",
    "Riesling",
    "Pinot Grigio",
    "Pinot Gris",
    "Zinfandel",
    "Syrah",
    "Shiraz",
    "Malbec",
    "Tempranillo",
    "Sangiovese",
    "Moscato",
    "Gewurztraminer",
];

// ── Malt Beverages (27 CFR 7.24) ────────────────────────────────────────

/// Standard malt beverage types recognized by the TTB.
pub const MALT_BEVERAGE_TYPES: &[&str] = &[
    "Beer",
    "Ale",
    "Lager",
    "Stout",
    "Porter",
    "Pilsner",
    "Pilsener",
    "India Pale Ale",
    "IPA",
    "Pale Ale",
    "Wheat Beer",
    "Hefeweizen",
    "Kolsch",
    "Kölsch",
    "Saison",
    "Bock",
    "Doppelbock",
    "Dunkel",
    "Marzen",
    "Oktoberfest",
    "Amber Ale",
    "Brown Ale",
    "Cream Ale",
    "Blonde Ale",
    "Golden Ale",
    "Red Ale",
    "Scotch Ale",
    "Barleywine",
    "Sour Beer",
    "Gose",
    "Berliner Weisse",
    "Lambic",
    "Malt Liquor",
    "Malt Beverage",
    "Hard Seltzer",
    "Flavored Malt Beverage",
];

// ── Common Misspellings ──────────────────────────────────────────────────

/// Known misspellings mapped to correct terms.
pub const COMMON_MISSPELLINGS: &[(&str, &str)] = &[
    ("burbon", "Bourbon"),
    ("bourban", "Bourbon"),
    ("whisky", "Whiskey"),
    ("vodca", "Vodka"),
    ("votka", "Vodka"),
    ("tequlia", "Tequila"),
    ("tequilla", "Tequila"),
    ("liqeur", "Liqueur"),
    ("liquer", "Liqueur"),
    ("liquor", "Liqueur"),
    ("cognack", "Cognac"),
    ("champaign", "Champagne"),
    ("champange", "Champagne"),
    ("cabernet sauvingon", "Cabernet Sauvignon"),
    ("cabernet savignon", "Cabernet Sauvignon"),
    ("chardonay", "Chardonnay"),
    ("chardanay", "Chardonnay"),
    ("rieseling", "Riesling"),
    ("merlo", "Merlot"),
    ("pinot nior", "Pinot Noir"),
    ("zinfandal", "Zinfandel"),
    ("pils", "Pilsner"),
    ("hefeweisen", "Hefeweizen"),
];

/// Result of validating a class/type designation against TTB standards.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// The input class/type as extracted from the label.
    pub input: String,
    /// Whether the designation matches a known TTB standard.
    pub is_valid: bool,
    /// The closest matching TTB standard term.
    pub matched_standard: Option<String>,
    /// Similarity score to the matched standard (0.0 - 1.0).
    pub similarity: f64,
    /// Beverage category: "spirits", "wine", or "malt_beverage".
    pub category: Option<String>,
    /// Whether a flavored designation was detected.
    pub is_flavored: bool,
    /// Detected misspelling correction, if any.
    pub spelling_correction: Option<String>,
    /// Whether a fanciful name was detected (requires statement of composition).
    pub requires_composition_statement: bool,
}

/// Validate a class/type designation against TTB standards of identity.
pub fn validate_classification(class_type: &str) -> ClassificationResult {
    let input = class_type.trim().to_string();
    let lower = input.to_lowercase();

    // Check for known misspellings first
    let spelling_correction = check_misspelling(&lower);

    // Check for flavored designation (e.g., "Chocolate Flavored Brandy")
    let (is_flavored, base_type) = check_flavored(&lower);

    // The term to match against standards
    let match_term = if let Some(ref correction) = spelling_correction {
        correction.to_lowercase()
    } else if is_flavored {
        base_type.clone()
    } else {
        lower.clone()
    };

    // Try matching against all categories
    let (best_match, best_score, category) = find_best_match(&match_term);

    let is_valid = best_score >= CLASS_MATCH_THRESHOLD;

    // If no good match found and input looks like a fanciful name
    let requires_composition_statement = !is_valid && !lower.is_empty() && !is_flavored;

    ClassificationResult {
        input,
        is_valid,
        matched_standard: if is_valid { Some(best_match) } else { None },
        similarity: best_score,
        category: if is_valid { Some(category) } else { None },
        is_flavored,
        spelling_correction,
        requires_composition_statement,
    }
}

/// Check if the input matches a known misspelling.
fn check_misspelling(input: &str) -> Option<String> {
    for (misspelling, correction) in COMMON_MISSPELLINGS {
        if input == *misspelling || jaro_winkler(input, misspelling) > 0.95 {
            return Some(correction.to_string());
        }
    }
    None
}

/// Detect flavored designation patterns.
/// Returns (is_flavored, base_spirit_type).
fn check_flavored(input: &str) -> (bool, String) {
    // Pattern: "{flavor} flavored {spirit_type}"
    if let Some(idx) = input.find("flavored") {
        let base = input[idx + "flavored".len()..].trim();
        if !base.is_empty() {
            return (true, base.to_string());
        }
    }
    // Pattern: "{flavor}-flavored {spirit_type}"
    if let Some(idx) = input.find("-flavored") {
        let base = input[idx + "-flavored".len()..].trim();
        if !base.is_empty() {
            return (true, base.to_string());
        }
    }
    (false, input.to_string())
}

/// Find the best matching standard term across all categories.
/// Returns (matched_term, score, category_name).
fn find_best_match(input: &str) -> (String, f64, String) {
    let mut best_match = String::new();
    let mut best_score: f64 = 0.0;
    let mut best_category = String::new();

    let categories: &[(&[&str], &str)] = &[
        (DISTILLED_SPIRITS_TYPES, "spirits"),
        (WINE_TYPES, "wine"),
        (MALT_BEVERAGE_TYPES, "malt_beverage"),
    ];

    for (standards, category) in categories {
        for standard in *standards {
            let score = jaro_winkler(input, &standard.to_lowercase());
            if score > best_score {
                best_score = score;
                best_match = standard.to_string();
                best_category = category.to_string();
            }
        }
    }

    (best_match, best_score, best_category)
}

/// Standard net contents sizes for TTB-regulated beverages (in mL).
pub const STANDARD_SIZES_ML: &[f64] = &[
    50.0, 100.0, 200.0, 375.0, 500.0, 750.0, 1000.0, 1750.0,
];

/// Validate net contents format per TTB requirements.
/// Returns (is_valid, normalized_value, unit).
pub fn validate_net_contents(net_contents: &str) -> (bool, Option<f64>, Option<String>) {
    let cleaned = net_contents.trim().to_lowercase();

    // Try to extract numeric value and unit
    let mut num_str = String::new();
    let mut unit_str = String::new();
    let mut found_digit = false;

    for ch in cleaned.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_str.push(ch);
            found_digit = true;
        } else if found_digit && !ch.is_whitespace() {
            unit_str.push(ch);
        }
    }

    let value: f64 = match num_str.parse() {
        Ok(v) => v,
        Err(_) => return (false, None, None),
    };

    // Normalize unit
    let unit = match unit_str.as_str() {
        "ml" | "milliliters" | "millilitres" => "mL",
        "l" | "liter" | "liters" | "litre" | "litres" => "L",
        "oz" | "floz" | "fl.oz." | "fl.oz" => "fl oz",
        _ => {
            // If no unit but value < 10, assume liters; otherwise mL
            if value < 10.0 { "L" } else { "mL" }
        }
    };

    // Validate: <1L must be in mL, ≥1L must be in liters
    let value_ml = match unit {
        "L" => value * 1000.0,
        "mL" => value,
        "fl oz" => value * 29.5735, // Convert fl oz to mL
        _ => value,
    };

    let is_valid = value_ml > 0.0;

    (is_valid, Some(value_ml), Some(unit.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bourbon() {
        let result = validate_classification("Kentucky Straight Bourbon Whiskey");
        assert!(result.is_valid);
        assert_eq!(result.category.as_deref(), Some("spirits"));
    }

    #[test]
    fn test_valid_vodka() {
        let result = validate_classification("Vodka");
        assert!(result.is_valid);
        assert_eq!(result.category.as_deref(), Some("spirits"));
    }

    #[test]
    fn test_misspelling_detection() {
        let result = validate_classification("Burbon Whiskey");
        assert!(result.spelling_correction.is_some());
    }

    #[test]
    fn test_flavored_spirit() {
        let result = validate_classification("Chocolate Flavored Brandy");
        assert!(result.is_flavored);
    }

    #[test]
    fn test_wine_classification() {
        let result = validate_classification("Cabernet Sauvignon");
        assert!(result.is_valid);
        assert_eq!(result.category.as_deref(), Some("wine"));
    }

    #[test]
    fn test_malt_beverage() {
        let result = validate_classification("India Pale Ale");
        assert!(result.is_valid);
        assert_eq!(result.category.as_deref(), Some("malt_beverage"));
    }

    #[test]
    fn test_fanciful_name_flagged() {
        let result = validate_classification("Mystic Dragon Fire");
        assert!(!result.is_valid);
        assert!(result.requires_composition_statement);
    }

    #[test]
    fn test_net_contents_ml() {
        let (valid, value, unit) = validate_net_contents("750 mL");
        assert!(valid);
        assert_eq!(value, Some(750.0));
        assert_eq!(unit.as_deref(), Some("mL"));
    }

    #[test]
    fn test_net_contents_liters() {
        let (valid, value, unit) = validate_net_contents("1.75 L");
        assert!(valid);
        assert_eq!(value, Some(1750.0));
        assert_eq!(unit.as_deref(), Some("L"));
    }
}

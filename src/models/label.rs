use garde::Validate;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// TTB beverage class designations per 27 CFR Parts 4, 5, 7.
#[derive(Debug, Clone, Serialize, Deserialize, EnumString, Display, PartialEq)]
#[strum(serialize_all = "title_case")]
pub enum BeverageClass {
    Wine,
    DistilledSpirits,
    MaltBeverage,
}

/// Fields extracted from a label image via Workers AI LLaVA.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ExtractedLabelFields {
    #[garde(length(min = 1, max = 200))]
    pub brand_name: String,

    #[garde(length(min = 1, max = 200))]
    pub class_type: String,

    #[garde(range(min = 0.0, max = 100.0))]
    pub abv: f64,

    #[garde(length(min = 1, max = 100))]
    pub net_contents: String,

    #[garde(skip)]
    pub country_of_origin: Option<String>,

    #[garde(skip)]
    pub government_warning: Option<String>,
}

/// Result of verifying extracted label fields against TTB rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub field_results: Vec<FieldVerification>,
    pub confidence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldVerification {
    pub field_name: String,
    pub expected: Option<String>,
    pub extracted: String,
    pub matches: bool,
    pub similarity_score: f64,
}

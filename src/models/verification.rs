use garde::Validate;
use serde::{Deserialize, Serialize};

/// Request to submit a label for verification (metadata portion).
#[derive(Debug, Deserialize, Validate)]
pub struct VerifyRequest {
    #[garde(length(min = 1, max = 200))]
    pub brand_name: Option<String>,

    #[garde(length(min = 1, max = 200))]
    pub class_type: Option<String>,

    #[garde(range(min = 0.0, max = 100.0))]
    pub expected_abv: Option<f64>,
}

/// Response after submitting a label for verification.
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub job_id: uuid::Uuid,
    pub status: String,
    pub message: String,
}

/// Response for querying job status.
#[derive(Debug, Serialize)]
pub struct JobStatusResponse {
    pub job_id: uuid::Uuid,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

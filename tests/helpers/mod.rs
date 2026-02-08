//! Test helper utilities for E2E testing

use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::fixtures::TestLabelFixture;

/// Response from POST /api/v1/verify
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub job_id: Uuid,
    pub status: String,
}

/// Response from GET /api/v1/verify/{job_id}
#[derive(Debug, Serialize, Deserialize)]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Field-level verification result
#[derive(Debug, Serialize, Deserialize)]
pub struct FieldVerificationResult {
    pub field_name: String,
    pub expected: Option<String>,
    pub extracted: String,
    pub passed: bool,
    pub similarity_score: Option<f64>,
    pub notes: Option<String>,
}

/// Complete verification result
#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub confidence_score: f64,
    pub field_results: Vec<FieldVerificationResult>,
    pub match_type: String,
    pub match_confidence: f64,
    pub matched_beverage_id: Option<Uuid>,
    pub warnings: Vec<String>,
}

/// Upload an image to the verify endpoint
pub async fn upload_label_image(
    client: &reqwest::Client,
    base_url: &str,
    image_path: &Path,
    brand: Option<&str>,
    class_type: Option<&str>,
    expected_abv: Option<f64>,
) -> Result<VerifyResponse, Box<dyn std::error::Error>> {
    let image_bytes = std::fs::read(image_path)?;
    let filename = image_path.file_name().unwrap().to_str().unwrap();

    let mut form = multipart::Form::new().part(
        "image",
        multipart::Part::bytes(image_bytes)
            .file_name(filename.to_string())
            .mime_str("image/png")?,
    );

    if let Some(b) = brand {
        form = form.text("brand_name", b.to_string());
    }
    if let Some(c) = class_type {
        form = form.text("class_type", c.to_string());
    }
    if let Some(a) = expected_abv {
        form = form.text("expected_abv", a.to_string());
    }

    let response = client
        .post(format!("{}/api/v1/verify", base_url))
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response.text().await?;
        return Err(format!("Upload failed with status {}: {}", status, error_text).into());
    }

    let body = response.json::<VerifyResponse>().await?;
    Ok(body)
}

/// Poll job status until completed or failed (with timeout)
pub async fn poll_job_status(
    client: &reqwest::Client,
    base_url: &str,
    job_id: &str,
    timeout_secs: u64,
) -> Result<JobStatusResponse, Box<dyn std::error::Error>> {
    let max_attempts = timeout_secs * 2; // Poll every 500ms

    for attempt in 0..max_attempts {
        let response = client
            .get(format!("{}/api/v1/verify/{}", base_url, job_id))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Status check failed: {}", error_text).into());
        }

        let status_response = response.json::<JobStatusResponse>().await?;

        match status_response.status.as_str() {
            "completed" | "failed" => return Ok(status_response),
            "pending" | "processing" => {
                if attempt % 10 == 0 && attempt > 0 {
                    println!("  ... still waiting (attempt {}/{})", attempt, max_attempts);
                }
                sleep(Duration::from_millis(500)).await;
            }
            _ => {
                return Err(format!("Unknown job status: {}", status_response.status).into());
            }
        }
    }

    Err(format!("Job did not complete within {} seconds", timeout_secs).into())
}

/// Wait for worker to process job (with timeout)
pub async fn wait_for_job_completion(
    client: &reqwest::Client,
    base_url: &str,
    job_id: &str,
) -> Result<JobStatusResponse, Box<dyn std::error::Error>> {
    poll_job_status(client, base_url, job_id, 120).await
}

/// Assert verification result matches expectations
pub fn assert_verification_result(
    result: &VerificationResult,
    fixture: &TestLabelFixture,
    strict: bool,
) {
    if strict {
        assert_eq!(
            result.passed, fixture.should_pass,
            "Verification passed status mismatch for {} (expected: {}, got: {})",
            fixture.filename, fixture.should_pass, result.passed
        );
    }

    // Check confidence score is reasonable
    assert!(
        result.confidence_score >= 0.0 && result.confidence_score <= 1.0,
        "Confidence score out of range: {} for {}",
        result.confidence_score,
        fixture.filename
    );

    // Find brand_name field result
    let brand_result = result
        .field_results
        .iter()
        .find(|f| f.field_name == "brand_name");

    if let Some(brand) = brand_result {
        if strict && fixture.expected_brand != "TBD" {
            assert!(
                brand.similarity_score.unwrap_or(0.0) >= 0.5,
                "Brand name similarity too low: {} for {}",
                brand.similarity_score.unwrap_or(0.0),
                fixture.filename
            );
        }
    }

    println!(
        "  ✓ {} - passed: {}, confidence: {:.2}",
        fixture.filename, result.passed, result.confidence_score
    );
}

/// Parse verification result from job status response
pub fn parse_verification_result(
    job_status: &JobStatusResponse,
) -> Result<VerificationResult, Box<dyn std::error::Error>> {
    let result_value = job_status
        .result
        .as_ref()
        .ok_or("No result in job status")?;

    let result: VerificationResult = serde_json::from_value(result_value.clone())?;
    Ok(result)
}

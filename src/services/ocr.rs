use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::models::label::ExtractedLabelFields;

/// Client for Cloudflare Workers AI LLaVA model.
pub struct WorkersAiClient {
    http: Client,
    account_id: String,
    api_token: String,
}

#[derive(Serialize)]
struct LlavaRequest {
    image: Vec<u8>,
    prompt: String,
    max_tokens: u32,
}

#[derive(Deserialize)]
struct LlavaResponse {
    result: Option<LlavaResult>,
    success: Option<bool>,
    errors: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct LlavaResult {
    description: Option<String>,
}

/// Lenient intermediate struct for LLaVA output where ABV may be a string.
#[derive(Deserialize)]
struct RawLlavaFields {
    brand_name: String,
    class_type: String,
    abv: String,
    net_contents: String,
    country_of_origin: Option<String>,
    government_warning: Option<String>,
}

impl WorkersAiClient {
    pub fn new(account_id: &str, api_token: &str) -> Result<Self, OcrError> {
        Ok(Self {
            http: Client::new(),
            account_id: account_id.to_string(),
            api_token: api_token.to_string(),
        })
    }

    /// Send a label image to Workers AI LLaVA and extract structured fields.
    pub async fn extract_label_fields(
        &self,
        image_bytes: &[u8],
    ) -> Result<ExtractedLabelFields, OcrError> {
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/@cf/llava-hf/llava-1.5-7b-hf",
            self.account_id
        );

        let prompt = concat!(
            "Analyze this beverage label image and extract the following fields as JSON: ",
            "brand_name, class_type (e.g. Wine, Distilled Spirits, Malt Beverage), ",
            "abv (alcohol by volume as a number), net_contents, ",
            "country_of_origin, government_warning. ",
            "Return ONLY valid JSON with these exact field names."
        );

        // Workers AI LLaVA expects image as raw byte array [u8], not base64
        let image_array: Vec<u8> = image_bytes.to_vec();
        let request_body = LlavaRequest {
            image: image_array,
            prompt: prompt.to_string(),
            max_tokens: 512,
        };

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&request_body)
            .send()
            .await
            .map_err(OcrError::Http)?;

        let status = response.status();
        let body = response.text().await.map_err(OcrError::Http)?;

        tracing::info!(status = %status, body_len = body.len(), "Workers AI response received");
        tracing::debug!(body = %body, "Workers AI raw response");

        if !status.is_success() {
            return Err(OcrError::Api(format!(
                "Workers AI returned HTTP {}: {}",
                status, body
            )));
        }

        let llava_resp: LlavaResponse = serde_json::from_str(&body)
            .map_err(|e| OcrError::Api(format!("Failed to parse response: {} — body: {}", e, body)))?;

        if let Some(false) = llava_resp.success {
            let errors = llava_resp.errors.unwrap_or_default();
            return Err(OcrError::Api(format!("Workers AI error: {:?}", errors)));
        }

        let description = llava_resp
            .result
            .and_then(|r| r.description)
            .ok_or_else(|| OcrError::Api(format!("No description in response: {}", body)))?;

        tracing::info!(description_len = description.len(), description = %description, "LLaVA description extracted");

        // Clean LLM output: strip Markdown-escaped underscores, trim whitespace
        let cleaned = description
            .replace("\\_", "_")
            .trim()
            .to_string();

        // Try parsing into a lenient intermediate format first
        let raw: RawLlavaFields = serde_json::from_str(&cleaned)
            .map_err(|e| OcrError::Api(format!("JSON parse error: {} — cleaned text: {}", e, cleaned)))?;

        // Convert ABV from string like "13.5%" to f64
        let abv = raw.abv
            .trim_end_matches('%')
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);

        Ok(ExtractedLabelFields {
            brand_name: raw.brand_name,
            class_type: raw.class_type,
            abv,
            net_contents: raw.net_contents,
            country_of_origin: raw.country_of_origin,
            government_warning: raw.government_warning,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Workers AI API error: {0}")]
    Api(String),

    #[error("Failed to parse LLaVA response as structured fields: {0}")]
    Parse(#[from] serde_json::Error),
}

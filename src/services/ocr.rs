use base64::Engine;
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
    result: LlavaResult,
}

#[derive(Deserialize)]
struct LlavaResult {
    description: String,
}

impl WorkersAiClient {
    pub fn new(account_id: String, api_token: String) -> Self {
        Self {
            http: Client::new(),
            account_id,
            api_token,
        }
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

        let request_body = serde_json::json!({
            "image": base64::engine::general_purpose::STANDARD.encode(image_bytes),
            "prompt": prompt,
            "max_tokens": 512
        });

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.api_token)
            .json(&request_body)
            .send()
            .await
            .map_err(OcrError::Http)?;

        let llava_resp: LlavaResponse = response.json().await.map_err(OcrError::Http)?;

        serde_json::from_str(&llava_resp.result.description).map_err(OcrError::Parse)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OcrError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to parse LLaVA response as structured fields: {0}")]
    Parse(#[from] serde_json::Error),
}

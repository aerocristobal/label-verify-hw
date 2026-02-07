use image::imageops::FilterType;
use image::ImageFormat;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

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

    /// Resize image if it exceeds Workers AI limits (~1MB as JSON array).
    /// Target: max 1024px on longest edge, JPEG quality 85.
    fn resize_if_needed(&self, image_bytes: &[u8]) -> Result<Vec<u8>, OcrError> {
        const MAX_DIMENSION: u32 = 1024;
        const JPEG_QUALITY: u8 = 85;

        // Load image
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| OcrError::ImageProcessing(format!("Failed to decode image: {}", e)))?;

        let width = img.width();
        let height = img.height();
        let max_side = width.max(height);

        // If already small enough, return as-is
        if max_side <= MAX_DIMENSION && image_bytes.len() < 800_000 {
            return Ok(image_bytes.to_vec());
        }

        tracing::info!(
            original_size = image_bytes.len(),
            original_dims = ?(width, height),
            "Resizing image for Workers AI"
        );

        // Calculate new dimensions maintaining aspect ratio
        let (new_width, new_height) = if width > height {
            (MAX_DIMENSION, (height as f64 * MAX_DIMENSION as f64 / width as f64) as u32)
        } else {
            ((width as f64 * MAX_DIMENSION as f64 / height as f64) as u32, MAX_DIMENSION)
        };

        // Resize with high-quality filter
        let resized = img.resize(new_width, new_height, FilterType::Lanczos3);

        // Re-encode as JPEG
        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);
        resized
            .write_to(&mut cursor, ImageFormat::Jpeg)
            .map_err(|e| OcrError::ImageProcessing(format!("Failed to encode JPEG: {}", e)))?;

        tracing::info!(
            resized_size = buf.len(),
            resized_dims = ?(new_width, new_height),
            compression_ratio = format!("{:.1}%", (buf.len() as f64 / image_bytes.len() as f64) * 100.0),
            "Image resized successfully"
        );

        Ok(buf)
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

        // Resize image if needed to avoid Workers AI payload size limits
        let processed_bytes = self.resize_if_needed(image_bytes)?;

        // Workers AI LLaVA expects image as raw byte array [u8], not base64
        let image_array: Vec<u8> = processed_bytes;
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

    #[error("Image processing error: {0}")]
    ImageProcessing(String),

    #[error("Failed to parse LLaVA response as structured fields: {0}")]
    Parse(#[from] serde_json::Error),
}

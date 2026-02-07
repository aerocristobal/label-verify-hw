use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::app_state::AppState;
use crate::db::queries;
use crate::models::job::JobStatus;
use crate::models::verification::{JobStatusResponse, VerifyResponse};
use crate::services::queue::QueuedJob;

const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MIN_IMAGE_SIZE: usize = 1024; // 1KB

/// POST /api/v1/verify — Upload a label image for verification.
pub async fn submit_verification(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<VerifyResponse>, (StatusCode, String)> {
    // Extract the image file from multipart upload
    let mut image_data: Option<Vec<u8>> = None;
    let mut metadata_brand: Option<String> = None;
    let mut metadata_class: Option<String> = None;
    let mut metadata_abv: Option<f64> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart error: {}", e)))?
    {
        match field.name() {
            Some("image") => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read image: {}", e)))?;

                // Validate size
                if data.len() < MIN_IMAGE_SIZE {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "Image too small (minimum 1KB)".to_string(),
                    ));
                }
                if data.len() > MAX_IMAGE_SIZE {
                    return Err((
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "Image too large (maximum 10MB)".to_string(),
                    ));
                }

                // Validate image format (JPEG, PNG, WebP)
                match image::guess_format(&data) {
                    Ok(format) => {
                        use image::ImageFormat;
                        match format {
                            ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP => {}
                            _ => {
                                return Err((
                                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                                    "Only JPEG, PNG, and WebP formats are supported".to_string(),
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        return Err((
                            StatusCode::UNSUPPORTED_MEDIA_TYPE,
                            "Invalid or unrecognized image format".to_string(),
                        ));
                    }
                }

                image_data = Some(data.to_vec());
            }
            Some("brand_name") => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid brand_name field".to_string()))?;
                metadata_brand = Some(text);
            }
            Some("class_type") => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid class_type field".to_string()))?;
                metadata_class = Some(text);
            }
            Some("expected_abv") => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid expected_abv field".to_string()))?;
                let abv = text
                    .parse::<f64>()
                    .map_err(|_| (StatusCode::BAD_REQUEST, "expected_abv must be a number".to_string()))?;
                metadata_abv = Some(abv);
            }
            _ => {}
        }
    }

    let image_data = image_data.ok_or((
        StatusCode::BAD_REQUEST,
        "Missing 'image' field in multipart upload".to_string(),
    ))?;

    // Encrypt the image using AES-256-GCM
    let encrypted_image = state
        .encryption
        .encrypt(&image_data)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Encryption failed: {}", e)))?;

    // Generate unique R2 storage key
    let job_id = Uuid::new_v4();
    let image_key = format!("images/{}.enc", job_id);

    // Upload encrypted image to R2
    state
        .storage
        .upload(&image_key, &encrypted_image, "application/octet-stream")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage upload failed: {}", e)))?;

    // Create job record in database
    let job = queries::create_job(&state.db, &image_key, None)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?;

    // Enqueue job for processing
    let queued_job = QueuedJob {
        job_id: job.id,
        image_key: image_key.clone(),
        expected_brand: metadata_brand,
        expected_class: metadata_class,
        expected_abv: metadata_abv,
    };

    state
        .queue
        .enqueue(&queued_job)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Queue error: {}", e)))?;

    tracing::info!(
        job_id = %job_id,
        image_key = %image_key,
        image_size = image_data.len(),
        encrypted_size = encrypted_image.len(),
        "Label verification job created and queued"
    );

    Ok(Json(VerifyResponse {
        job_id,
        status: "pending".to_string(),
        message: "Label submitted for verification".to_string(),
    }))
}

/// GET /api/v1/verify/:job_id — Check verification job status.
pub async fn get_job_status(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> Result<Json<JobStatusResponse>, (StatusCode, String)> {
    // Look up job in PostgreSQL
    let job = queries::get_job(&state.db, job_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)))?
        .ok_or((StatusCode::NOT_FOUND, "Job not found".to_string()))?;

    let status_str = match job.status {
        JobStatus::Pending => "pending",
        JobStatus::Processing => "processing",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    };

    tracing::info!(
        job_id = %job_id,
        status = status_str,
        "Job status retrieved"
    );

    Ok(Json(JobStatusResponse {
        job_id: job.id,
        status: status_str.to_string(),
        result: job.result,
        error: job.error,
    }))
}

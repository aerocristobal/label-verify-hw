use axum::extract::Multipart;
use axum::http::StatusCode;
use axum::Json;
use uuid::Uuid;

use crate::models::verification::{VerifyResponse, JobStatusResponse};

/// POST /api/v1/verify — Upload a label image for verification.
pub async fn submit_verification(
    mut multipart: Multipart,
) -> Result<Json<VerifyResponse>, StatusCode> {
    // Extract the image file from multipart upload
    let mut _image_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() == Some("image") {
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

            // Validate image format using the `image` crate
            image::guess_format(&data).map_err(|_| StatusCode::UNSUPPORTED_MEDIA_TYPE)?;

            _image_data = Some(data.to_vec());
        }
    }

    let _image_data = _image_data.ok_or(StatusCode::BAD_REQUEST)?;

    // TODO: Encrypt image, upload to R2, create DB record, enqueue job
    let job_id = Uuid::new_v4();

    Ok(Json(VerifyResponse {
        job_id,
        status: "pending".to_string(),
        message: "Label submitted for verification".to_string(),
    }))
}

/// GET /api/v1/verify/:job_id — Check verification job status.
pub async fn get_job_status(
    axum::extract::Path(job_id): axum::extract::Path<Uuid>,
) -> Result<Json<JobStatusResponse>, StatusCode> {
    // TODO: Look up job in PostgreSQL
    Ok(Json(JobStatusResponse {
        job_id,
        status: "pending".to_string(),
        result: None,
        error: None,
    }))
}

use label_verify_hw::{
    app_state::AppState,
    config::AppConfig,
    db::{self, queries},
    models::job::JobStatus,
    services::{
        encryption::EncryptionService, ocr::WorkersAiClient, queue::JobQueue, storage::R2Client,
        validation,
    },
};
use std::time::Duration;
use tokio::time::sleep;
use tracing_subscriber::EnvFilter;

const MAX_RETRIES: i32 = 3;
const POLL_INTERVAL_MS: u64 = 1000; // 1 second

#[tokio::main]
async fn main() {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .json()
        .init();

    tracing::info!("Starting label verification worker");

    // Load configuration
    let config = AppConfig::from_env().expect("Failed to load configuration");

    // Initialize database
    tracing::info!("Connecting to PostgreSQL");
    let db_pool = db::init_pool(&config.database_url)
        .await
        .expect("Failed to connect to database");

    // Initialize services
    tracing::info!("Initializing services");
    let r2_client =
        R2Client::new(
            &config.r2_bucket,
            &config.r2_endpoint,
            &config.r2_access_key,
            &config.r2_secret_key,
        )
        .expect("Failed to initialize R2 client");

    let encryption =
        EncryptionService::new(&config.encryption_key).expect("Failed to initialize encryption");

    let queue = JobQueue::new(&config.redis_url).expect("Failed to initialize job queue");

    let ocr_client = WorkersAiClient::new(&config.cf_account_id, &config.cf_api_token)
        .expect("Failed to initialize Workers AI client");

    let state = AppState::new(db_pool, r2_client, encryption, queue, ocr_client);

    tracing::info!("Worker ready, starting job processing loop");

    // Main processing loop
    loop {
        match process_next_job(&state).await {
            Ok(true) => {
                // Job processed successfully, continue immediately
                tracing::debug!("Job processed, checking for next job");
            }
            Ok(false) => {
                // No job available, sleep before next poll
                tracing::trace!("No jobs available, sleeping");
                sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
            }
            Err(e) => {
                tracing::error!(error = %e, "Error processing job, will retry");
                sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
            }
        }
    }
}

/// Process the next job from the queue.
/// Returns Ok(true) if a job was processed, Ok(false) if no job available.
async fn process_next_job(state: &AppState) -> Result<bool, Box<dyn std::error::Error>> {
    // Dequeue next job
    let job = match state.queue.dequeue().await? {
        Some(j) => j,
        None => return Ok(false), // No job available
    };

    tracing::info!(
        job_id = %job.job_id,
        image_key = %job.image_key,
        "Processing verification job"
    );

    // Update job status to processing
    if let Err(e) = queries::update_job_status(&state.db, job.job_id, JobStatus::Processing).await
    {
        tracing::error!(job_id = %job.job_id, error = %e, "Failed to update job status");
        return Err(e.into());
    }

    // Process the job
    match process_job_inner(state, &job).await {
        Ok(result) => {
            // Store results in database
            let result_json = serde_json::to_value(&result)?;
            queries::update_job_result(
                &state.db,
                job.job_id,
                JobStatus::Completed,
                Some(result_json),
                None,
            )
            .await?;

            // Mark job as complete in queue
            state.queue.complete(&job).await?;

            tracing::info!(
                job_id = %job.job_id,
                passed = result.passed,
                confidence = result.confidence_score,
                "Job completed successfully"
            );

            Ok(true)
        }
        Err(e) => {
            tracing::error!(job_id = %job.job_id, error = %e, "Job processing failed");

            // Check retry count
            let retry_count = queries::increment_retry_count(&state.db, job.job_id).await?;

            if retry_count >= MAX_RETRIES {
                // Max retries exceeded, mark as failed
                queries::update_job_result(
                    &state.db,
                    job.job_id,
                    JobStatus::Failed,
                    None,
                    Some(&format!("Processing failed after {} retries: {}", MAX_RETRIES, e)),
                )
                .await?;

                state.queue.complete(&job).await?;

                tracing::warn!(
                    job_id = %job.job_id,
                    retry_count = retry_count,
                    "Job failed after max retries"
                );
            } else {
                // Re-queue for retry
                state.queue.enqueue(&job).await?;
                state.queue.complete(&job).await?;

                queries::update_job_status(&state.db, job.job_id, JobStatus::Pending).await?;

                tracing::info!(
                    job_id = %job.job_id,
                    retry_count = retry_count,
                    "Job re-queued for retry"
                );
            }

            Ok(true)
        }
    }
}

/// Inner job processing logic.
async fn process_job_inner(
    state: &AppState,
    job: &label_verify_hw::services::queue::QueuedJob,
) -> Result<label_verify_hw::models::label::VerificationResult, Box<dyn std::error::Error>> {
    // Download encrypted image from R2
    tracing::debug!(job_id = %job.job_id, "Downloading image from R2");
    let encrypted_image = state.storage.download(&job.image_key).await?;

    // Decrypt image in memory
    tracing::debug!(job_id = %job.job_id, "Decrypting image");
    let image_bytes = state.encryption.decrypt(&encrypted_image)?;

    // Call Workers AI for OCR
    tracing::debug!(job_id = %job.job_id, "Calling Workers AI for OCR");
    let start = std::time::Instant::now();
    let extracted_fields = state.ocr.extract_label_fields(&image_bytes).await?;
    let ocr_duration = start.elapsed();

    tracing::info!(
        job_id = %job.job_id,
        ocr_duration_ms = ocr_duration.as_millis(),
        brand = %extracted_fields.brand_name,
        class = %extracted_fields.class_type,
        abv = extracted_fields.abv,
        "OCR extraction complete"
    );

    // Validate extracted fields
    tracing::debug!(job_id = %job.job_id, "Validating fields");
    let verification_result = validation::verify_label(
        &extracted_fields,
        job.expected_brand.as_deref(),
        job.expected_class.as_deref(),
        job.expected_abv,
    );

    tracing::info!(
        job_id = %job.job_id,
        passed = verification_result.passed,
        confidence = verification_result.confidence_score,
        issues_count = verification_result.field_results.iter().filter(|f| !f.matches).count(),
        "Validation complete"
    );

    Ok(verification_result)
}

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::job::{JobStatus, VerificationJob};

/// Insert a new verification job
pub async fn create_job(
    pool: &PgPool,
    image_key: &str,
    user_id: Option<&str>,
) -> Result<VerificationJob, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        INSERT INTO verification_jobs (status, image_key, user_id)
        VALUES ('pending', $1, $2)
        RETURNING id, status, image_key, created_at, updated_at, retry_count, error,
                  extracted_fields, verification_result
        "#,
        image_key,
        user_id
    )
    .fetch_one(pool)
    .await?;

    Ok(VerificationJob {
        id: row.id,
        status: JobStatus::Pending,
        image_key: row.image_key,
        created_at: row.created_at,
        updated_at: row.updated_at,
        result: row.verification_result,
        error: row.error,
        retry_count: row.retry_count,
    })
}

/// Get a job by ID
pub async fn get_job(pool: &PgPool, job_id: Uuid) -> Result<Option<VerificationJob>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT id, status, image_key, created_at, updated_at, retry_count, error,
               extracted_fields, verification_result
        FROM verification_jobs
        WHERE id = $1
        "#,
        job_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| {
        let status = match r.status.as_str() {
            "pending" => JobStatus::Pending,
            "processing" => JobStatus::Processing,
            "completed" => JobStatus::Completed,
            "failed" => JobStatus::Failed,
            _ => JobStatus::Pending,
        };

        VerificationJob {
            id: r.id,
            status,
            image_key: r.image_key,
            created_at: r.created_at,
            updated_at: r.updated_at,
            result: r.verification_result,
            error: r.error,
            retry_count: r.retry_count,
        }
    }))
}

/// Update job status
pub async fn update_job_status(
    pool: &PgPool,
    job_id: Uuid,
    status: JobStatus,
) -> Result<(), sqlx::Error> {
    let status_str = match status {
        JobStatus::Pending => "pending",
        JobStatus::Processing => "processing",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    };

    sqlx::query!(
        r#"
        UPDATE verification_jobs
        SET status = $1,
            processing_started_at = CASE WHEN $1 = 'processing' THEN NOW() ELSE processing_started_at END,
            processing_completed_at = CASE WHEN $1 IN ('completed', 'failed') THEN NOW() ELSE processing_completed_at END
        WHERE id = $2
        "#,
        status_str,
        job_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update job with results
pub async fn update_job_result(
    pool: &PgPool,
    job_id: Uuid,
    status: JobStatus,
    result: Option<serde_json::Value>,
    error: Option<&str>,
) -> Result<(), sqlx::Error> {
    let status_str = match status {
        JobStatus::Pending => "pending",
        JobStatus::Processing => "processing",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    };

    sqlx::query!(
        r#"
        UPDATE verification_jobs
        SET status = $1,
            verification_result = $2,
            error = $3,
            processing_completed_at = NOW()
        WHERE id = $4
        "#,
        status_str,
        result,
        error,
        job_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment retry count
pub async fn increment_retry_count(pool: &PgPool, job_id: Uuid) -> Result<i32, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        UPDATE verification_jobs
        SET retry_count = retry_count + 1
        WHERE id = $1
        RETURNING retry_count
        "#,
        job_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.retry_count)
}

/// Get pending jobs (for queue processor)
pub async fn get_pending_jobs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<VerificationJob>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT id, status, image_key, created_at, updated_at, retry_count, error,
               extracted_fields, verification_result
        FROM verification_jobs
        WHERE status = 'pending'
        ORDER BY created_at ASC
        LIMIT $1
        "#,
        limit
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| VerificationJob {
            id: r.id,
            status: JobStatus::Pending,
            image_key: r.image_key,
            created_at: r.created_at,
            updated_at: r.updated_at,
            result: r.verification_result,
            error: r.error,
            retry_count: r.retry_count,
        })
        .collect())
}

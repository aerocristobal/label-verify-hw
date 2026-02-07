use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::job::{JobStatus, VerificationJob};

/// Insert a new verification job
pub async fn create_job(
    pool: &PgPool,
    image_key: &str,
    user_id: Option<&str>,
) -> Result<VerificationJob, sqlx::Error> {
    let row = sqlx::query(
        r#"
        INSERT INTO verification_jobs (status, image_key, user_id)
        VALUES ('pending', $1, $2)
        RETURNING id, status, image_key, created_at, updated_at, retry_count, error,
                  extracted_fields, verification_result
        "#,
    )
    .bind(image_key)
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(VerificationJob {
        id: row.try_get("id")?,
        status: JobStatus::Pending,
        image_key: row.try_get("image_key")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        result: row.try_get("verification_result")?,
        error: row.try_get("error")?,
        retry_count: row.try_get("retry_count")?,
    })
}

/// Get a job by ID
pub async fn get_job(pool: &PgPool, job_id: Uuid) -> Result<Option<VerificationJob>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, status, image_key, created_at, updated_at, retry_count, error,
               extracted_fields, verification_result
        FROM verification_jobs
        WHERE id = $1
        "#,
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some(r) => {
            let status_str: String = r.try_get("status")?;
            let status = match status_str.as_str() {
                "pending" => JobStatus::Pending,
                "processing" => JobStatus::Processing,
                "completed" => JobStatus::Completed,
                "failed" => JobStatus::Failed,
                _ => JobStatus::Pending,
            };

            Some(VerificationJob {
                id: r.try_get("id")?,
                status,
                image_key: r.try_get("image_key")?,
                created_at: r.try_get("created_at")?,
                updated_at: r.try_get("updated_at")?,
                result: r.try_get("verification_result")?,
                error: r.try_get("error")?,
                retry_count: r.try_get("retry_count")?,
            })
        }
        None => None,
    })
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

    sqlx::query(
        r#"
        UPDATE verification_jobs
        SET status = $1,
            processing_started_at = CASE WHEN $1 = 'processing' THEN NOW() ELSE processing_started_at END,
            processing_completed_at = CASE WHEN $1 IN ('completed', 'failed') THEN NOW() ELSE processing_completed_at END
        WHERE id = $2
        "#,
    )
    .bind(status_str)
    .bind(job_id)
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

    sqlx::query(
        r#"
        UPDATE verification_jobs
        SET status = $1,
            verification_result = $2,
            error = $3,
            processing_completed_at = NOW()
        WHERE id = $4
        "#,
    )
    .bind(status_str)
    .bind(result)
    .bind(error)
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Increment retry count
pub async fn increment_retry_count(pool: &PgPool, job_id: Uuid) -> Result<i32, sqlx::Error> {
    let row = sqlx::query(
        r#"
        UPDATE verification_jobs
        SET retry_count = retry_count + 1
        WHERE id = $1
        RETURNING retry_count
        "#,
    )
    .bind(job_id)
    .fetch_one(pool)
    .await?;

    Ok(row.try_get("retry_count")?)
}

/// Get pending jobs (for queue processor)
pub async fn get_pending_jobs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<VerificationJob>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, status, image_key, created_at, updated_at, retry_count, error,
               extracted_fields, verification_result
        FROM verification_jobs
        WHERE status = 'pending'
        ORDER BY created_at ASC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(VerificationJob {
                id: r.try_get("id")?,
                status: JobStatus::Pending,
                image_key: r.try_get("image_key")?,
                created_at: r.try_get("created_at")?,
                updated_at: r.try_get("updated_at")?,
                result: r.try_get("verification_result")?,
                error: r.try_get("error")?,
                retry_count: r.try_get("retry_count")?,
            })
        })
        .collect()
}

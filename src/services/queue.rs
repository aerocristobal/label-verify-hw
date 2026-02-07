use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const QUEUE_KEY: &str = "label_verify:jobs";
const PROCESSING_KEY: &str = "label_verify:processing";

/// Job payload serialized into Redis.
#[derive(Debug, Serialize, Deserialize)]
pub struct QueuedJob {
    pub job_id: Uuid,
    pub image_key: String,
    pub expected_brand: Option<String>,
    pub expected_class: Option<String>,
    pub expected_abv: Option<f64>,
}

/// Redis-backed async job queue with retry support.
pub struct JobQueue {
    client: redis::Client,
}

impl JobQueue {
    pub fn new(redis_url: &str) -> Result<Self, QueueError> {
        let client = redis::Client::open(redis_url).map_err(QueueError::Redis)?;
        Ok(Self { client })
    }

    /// Enqueue a verification job.
    pub async fn enqueue(&self, job: &QueuedJob) -> Result<(), QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await.map_err(QueueError::Redis)?;
        let payload = serde_json::to_string(job).map_err(QueueError::Serialize)?;
        conn.lpush::<_, _, ()>(QUEUE_KEY, &payload)
            .await
            .map_err(QueueError::Redis)?;
        Ok(())
    }

    /// Dequeue a job for processing (blocking pop with move to processing set).
    pub async fn dequeue(&self) -> Result<Option<QueuedJob>, QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await.map_err(QueueError::Redis)?;
        let result: Option<String> = conn
            .rpoplpush(QUEUE_KEY, PROCESSING_KEY)
            .await
            .map_err(QueueError::Redis)?;

        match result {
            Some(payload) => {
                let job: QueuedJob = serde_json::from_str(&payload).map_err(QueueError::Serialize)?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    /// Check Redis connectivity (for health checks).
    pub async fn health_check(&self) -> Result<(), QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await.map_err(QueueError::Redis)?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(QueueError::Redis)?;
        Ok(())
    }

    /// Get the current queue depth (pending jobs).
    pub async fn queue_depth(&self) -> Result<u64, QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await.map_err(QueueError::Redis)?;
        let depth: u64 = conn.llen(QUEUE_KEY).await.map_err(QueueError::Redis)?;
        Ok(depth)
    }

    /// Mark a job as complete (remove from processing set).
    pub async fn complete(&self, job: &QueuedJob) -> Result<(), QueueError> {
        let mut conn = self.client.get_multiplexed_async_connection().await.map_err(QueueError::Redis)?;
        let payload = serde_json::to_string(job).map_err(QueueError::Serialize)?;
        conn.lrem::<_, _, ()>(PROCESSING_KEY, 1, &payload)
            .await
            .map_err(QueueError::Redis)?;
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
}

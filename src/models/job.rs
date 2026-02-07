use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Status of a label verification job in the async queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

/// A label verification job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationJob {
    pub id: Uuid,
    pub status: JobStatus,
    pub image_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub retry_count: i32,
}

use sqlx::PgPool;
use std::sync::Arc;

use crate::services::{
    encryption::EncryptionService,
    ocr::WorkersAiClient,
    queue::JobQueue,
    storage::R2Client,
};

/// Shared application state passed to all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub storage: Arc<R2Client>,
    pub encryption: Arc<EncryptionService>,
    pub queue: Arc<JobQueue>,
    pub ocr: Arc<WorkersAiClient>,
}

impl AppState {
    pub fn new(
        db: PgPool,
        storage: R2Client,
        encryption: EncryptionService,
        queue: JobQueue,
        ocr: WorkersAiClient,
    ) -> Self {
        Self {
            db,
            storage: Arc::new(storage),
            encryption: Arc::new(encryption),
            queue: Arc::new(queue),
            ocr: Arc::new(ocr),
        }
    }
}

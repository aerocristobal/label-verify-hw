use label_verify_hw::{
    app_state::AppState,
    config::AppConfig,
    db::{self, queries},
    models::job::JobStatus,
    services::{
        encryption::EncryptionService, ocr::WorkersAIClient, queue::JobQueue, storage::R2Client,
    },
};
use uuid::Uuid;

/// Integration test: Full verification flow
///
/// This test verifies the complete integration:
/// 1. Database connection and schema
/// 2. R2 storage (upload/download/delete)
/// 3. Encryption/decryption
/// 4. Job queue (enqueue/dequeue)
/// 5. Database operations (create/read/update jobs)
///
/// Note: This requires a running PostgreSQL and Redis instance
/// configured via environment variables.
#[tokio::test]
#[ignore] // Run with: cargo test --test integration_test -- --ignored
async fn test_full_integration() {
    // Load config from environment
    let config = AppConfig::from_env().expect("Failed to load config");

    // Initialize database
    let db_pool = db::init_pool(&config.database_url)
        .await
        .expect("Failed to connect to database");

    db::run_migrations(&db_pool)
        .await
        .expect("Failed to run migrations");

    // Initialize services
    let r2_client = R2Client::new(
        &config.r2_bucket,
        &config.r2_endpoint,
        &config.r2_access_key,
        &config.r2_secret_key,
    )
    .expect("Failed to initialize R2");

    let encryption =
        EncryptionService::new(&config.encryption_key).expect("Failed to initialize encryption");

    let queue = JobQueue::new(&config.redis_url).expect("Failed to initialize queue");

    let ocr_client = WorkersAIClient::new(&config.cf_account_id, &config.cf_api_token)
        .expect("Failed to initialize Workers AI");

    let state = AppState::new(db_pool.clone(), r2_client, encryption, queue, ocr_client);

    // Test data
    let test_image = b"fake image data for testing";
    let encrypted_image = state.encryption.encrypt(test_image).expect("Encryption failed");

    // 1. Test R2 upload
    let test_key = format!("test/{}.enc", Uuid::new_v4());
    state
        .storage
        .upload(&test_key, &encrypted_image, "application/octet-stream")
        .await
        .expect("R2 upload failed");

    // 2. Test database job creation
    let job = queries::create_job(&db_pool, &test_key, Some("test-user"))
        .await
        .expect("Failed to create job");

    assert_eq!(job.status, JobStatus::Pending);
    assert_eq!(job.image_key, test_key);
    assert_eq!(job.retry_count, 0);

    // 3. Test job retrieval
    let retrieved_job = queries::get_job(&db_pool, job.id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(retrieved_job.id, job.id);
    assert_eq!(retrieved_job.status, JobStatus::Pending);

    // 4. Test job status update
    queries::update_job_status(&db_pool, job.id, JobStatus::Processing)
        .await
        .expect("Failed to update status");

    let updated_job = queries::get_job(&db_pool, job.id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(updated_job.status, JobStatus::Processing);

    // 5. Test queue operations
    let queued_job = label_verify_hw::services::queue::QueuedJob {
        job_id: job.id,
        image_key: test_key.clone(),
        expected_brand: Some("Test Brand".to_string()),
        expected_class: Some("Wine".to_string()),
        expected_abv: Some(13.5),
    };

    state
        .queue
        .enqueue(&queued_job)
        .await
        .expect("Failed to enqueue");

    let dequeued = state
        .queue
        .dequeue()
        .await
        .expect("Failed to dequeue")
        .expect("No job in queue");

    assert_eq!(dequeued.job_id, job.id);
    assert_eq!(dequeued.image_key, test_key);

    // 6. Test R2 download
    let downloaded = state
        .storage
        .download(&test_key)
        .await
        .expect("R2 download failed");

    assert_eq!(downloaded, encrypted_image);

    // 7. Test decryption
    let decrypted = state
        .encryption
        .decrypt(&downloaded)
        .expect("Decryption failed");

    assert_eq!(decrypted, test_image);

    // 8. Test job completion
    let result = serde_json::json!({
        "passed": true,
        "confidence_score": 0.95,
        "field_results": []
    });

    queries::update_job_result(&db_pool, job.id, JobStatus::Completed, Some(result), None)
        .await
        .expect("Failed to update result");

    let final_job = queries::get_job(&db_pool, job.id)
        .await
        .expect("Failed to get job")
        .expect("Job not found");

    assert_eq!(final_job.status, JobStatus::Completed);
    assert!(final_job.result.is_some());

    // Cleanup
    state
        .storage
        .delete(&test_key)
        .await
        .expect("Failed to delete test file");

    state
        .queue
        .complete(&dequeued)
        .await
        .expect("Failed to complete job in queue");

    println!("✅ All integration tests passed!");
}

/// Test encryption/decryption round-trip
#[test]
fn test_encryption_roundtrip() {
    let key = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 32]);
    let encryption = EncryptionService::new(&key).expect("Failed to create encryption service");

    let plaintext = b"sensitive label image data";
    let encrypted = encryption.encrypt(plaintext).expect("Encryption failed");
    let decrypted = encryption.decrypt(&encrypted).expect("Decryption failed");

    assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    assert_ne!(encrypted, plaintext.to_vec()); // Should be different when encrypted
}

/// Test field validation logic
#[test]
fn test_validation_logic() {
    use label_verify_hw::models::label::ExtractedLabelFields;
    use label_verify_hw::services::validation;

    let extracted = ExtractedLabelFields {
        brand_name: "Test Wine Brand".to_string(),
        class_type: "Wine".to_string(),
        abv: 13.5,
        net_contents: "750ml".to_string(),
        country_of_origin: Some("USA".to_string()),
        government_warning: Some("Contains sulfites".to_string()),
    };

    // Test exact match
    let result = validation::verify_label(
        &extracted,
        Some("Test Wine Brand"),
        Some("Wine"),
        Some(13.5),
    );

    assert!(result.passed);
    assert!(result.confidence_score > 0.9);

    // Test fuzzy match
    let result_fuzzy = validation::verify_label(
        &extracted,
        Some("Test Winery Brand"), // Slightly different
        Some("Wine"),
        Some(13.0), // Slightly different ABV
    );

    // Should still pass with good confidence due to similarity
    assert!(result_fuzzy.confidence_score > 0.7);
}

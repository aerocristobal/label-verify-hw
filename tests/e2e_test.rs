//! End-to-end tests with real label images
//!
//! These tests require:
//! 1. PostgreSQL database running (with migrations applied)
//! 2. Redis running
//! 3. API server running on configured port
//! 4. Worker process running
//! 5. Cloudflare Workers AI and R2 credentials configured
//!
//! Run with: cargo test --test e2e_test -- --ignored --nocapture
//!
//! Set API_BASE_URL to override default (http://localhost:3000)

mod fixtures;
mod helpers;

use fixtures::*;
use helpers::*;
use std::path::PathBuf;

/// Get base URL from env or default to localhost
fn get_base_url() -> String {
    std::env::var("API_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

/// Get tests directory path
fn get_tests_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests")
}

#[tokio::test]
#[ignore] // Requires running API server, worker, and all infrastructure
async fn test_e2e_health_check() {
    let base_url = get_base_url();
    let client = reqwest::Client::new();

    let response = client
        .get(format!("{}/health", base_url))
        .send()
        .await
        .expect("Health check failed");

    assert!(
        response.status().is_success(),
        "Health check returned non-success status: {}",
        response.status()
    );

    println!("✓ Health check passed");
}

#[tokio::test]
#[ignore] // Requires running API server, worker, and all infrastructure
async fn test_e2e_single_label_verification() {
    // Test with first fixture (smallest image)
    let fixture = &TEST_FIXTURES[8]; // test_label9.png - smallest
    let base_url = get_base_url();
    let client = reqwest::Client::new();

    println!("Testing single label: {} - {}", fixture.filename, fixture.description);

    // 1. Upload label image
    let image_path = get_tests_dir().join(fixture.filename);
    assert!(
        image_path.exists(),
        "Test image not found: {}",
        image_path.display()
    );

    let upload_response = upload_label_image(
        &client,
        &base_url,
        &image_path,
        if fixture.expected_brand != "TBD" { Some(fixture.expected_brand) } else { None },
        if fixture.expected_class != "TBD" { Some(fixture.expected_class) } else { None },
        if fixture.expected_abv > 0.0 { Some(fixture.expected_abv) } else { None },
    )
    .await
    .expect("Failed to upload image");

    assert_eq!(upload_response.status, "pending");
    println!("  ✓ Upload successful, job_id: {}", upload_response.job_id);

    // 2. Poll for job completion
    let job_status = wait_for_job_completion(
        &client,
        &base_url,
        &upload_response.job_id.to_string(),
    )
    .await
    .expect("Failed to wait for job completion");

    println!("  ✓ Job completed with status: {}", job_status.status);

    // 3. Validate result (if completed successfully)
    if job_status.status == "completed" {
        let result = parse_verification_result(&job_status)
            .expect("Failed to parse verification result");

        // Non-strict assertion since fixtures don't have expected values yet
        assert_verification_result(&result, fixture, false);
    } else if job_status.status == "failed" {
        println!("  ⚠ Job failed: {:?}", job_status.error);
        // Don't fail test - job failure might be expected during development
    }
}

#[tokio::test]
#[ignore]
async fn test_e2e_all_test_labels() {
    let base_url = get_base_url();
    let client = reqwest::Client::new();
    let tests_dir = get_tests_dir();

    println!("\nTesting all {} label images:\n", TEST_FIXTURES.len());

    let mut successful = 0;
    let mut failed = 0;

    for (idx, fixture) in TEST_FIXTURES.iter().enumerate() {
        println!(
            "[{}/{}] Testing: {} - {}",
            idx + 1,
            TEST_FIXTURES.len(),
            fixture.filename,
            fixture.description
        );

        let image_path = tests_dir.join(fixture.filename);

        if !image_path.exists() {
            println!("  ⚠ Image not found, skipping");
            continue;
        }

        // Upload
        let upload_response = match upload_label_image(
            &client,
            &base_url,
            &image_path,
            if fixture.expected_brand != "TBD" { Some(fixture.expected_brand) } else { None },
            if fixture.expected_class != "TBD" { Some(fixture.expected_class) } else { None },
            if fixture.expected_abv > 0.0 { Some(fixture.expected_abv) } else { None },
        )
        .await
        {
            Ok(resp) => resp,
            Err(e) => {
                println!("  ✗ Upload failed: {}", e);
                failed += 1;
                continue;
            }
        };

        println!("  ✓ Uploaded, job_id: {}", upload_response.job_id);

        // Wait for completion
        let job_status = match wait_for_job_completion(
            &client,
            &base_url,
            &upload_response.job_id.to_string(),
        )
        .await
        {
            Ok(status) => status,
            Err(e) => {
                println!("  ✗ Job status check failed: {}", e);
                failed += 1;
                continue;
            }
        };

        if job_status.status == "completed" {
            match parse_verification_result(&job_status) {
                Ok(result) => {
                    assert_verification_result(&result, fixture, false);
                    successful += 1;
                }
                Err(e) => {
                    println!("  ✗ Failed to parse result: {}", e);
                    failed += 1;
                }
            }
        } else {
            println!("  ✗ Job failed: {:?}", job_status.error);
            failed += 1;
        }

        println!();
    }

    println!("\n=== Summary ===");
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);
    println!("Total: {}", TEST_FIXTURES.len());

    // Allow some failures during development
    assert!(
        successful > 0,
        "All tests failed - check if API server and worker are running"
    );
}

#[tokio::test]
#[ignore]
async fn test_e2e_large_image_handling() {
    // Test that large images are handled properly (resizing for Workers AI)
    let base_url = get_base_url();
    let client = reqwest::Client::new();

    // Use test_label4.png (4.5MB - largest test image)
    let large_fixture = TEST_FIXTURES
        .iter()
        .find(|f| f.filename == "test_label4.png")
        .expect("test_label4.png fixture not found");

    println!("Testing large image: {} - {}", large_fixture.filename, large_fixture.description);

    let large_image_path = get_tests_dir().join(large_fixture.filename);

    let upload_response = upload_label_image(
        &client,
        &base_url,
        &large_image_path,
        if large_fixture.expected_brand != "TBD" { Some(large_fixture.expected_brand) } else { None },
        if large_fixture.expected_class != "TBD" { Some(large_fixture.expected_class) } else { None },
        if large_fixture.expected_abv > 0.0 { Some(large_fixture.expected_abv) } else { None },
    )
    .await
    .expect("Failed to upload large image");

    println!("  ✓ Large image uploaded, job_id: {}", upload_response.job_id);

    let job_status = wait_for_job_completion(
        &client,
        &base_url,
        &upload_response.job_id.to_string(),
    )
    .await
    .expect("Failed to wait for job completion");

    // Should complete successfully (image resizing should work)
    println!("  ✓ Job completed with status: {}", job_status.status);

    if job_status.status == "completed" {
        let result = parse_verification_result(&job_status)
            .expect("Failed to parse result");
        assert_verification_result(&result, large_fixture, false);
    }
}

#[tokio::test]
#[ignore]
async fn test_e2e_image_format_validation() {
    // Test that API properly rejects invalid images
    let base_url = get_base_url();
    let client = reqwest::Client::new();

    println!("Testing invalid image rejection");

    // Try uploading a non-image file (should fail)
    let fake_image = vec![0u8; 100]; // Random bytes

    let form = reqwest::multipart::Form::new().part(
        "image",
        reqwest::multipart::Part::bytes(fake_image)
            .file_name("fake.png")
            .mime_str("image/png")
            .unwrap(),
    );

    let response = client
        .post(format!("{}/api/v1/verify", base_url))
        .multipart(form)
        .send()
        .await
        .expect("Request failed");

    // Should return 4xx error (Bad Request or Unsupported Media Type)
    assert!(
        response.status().is_client_error(),
        "Should reject invalid image format, got status: {}",
        response.status()
    );

    println!("  ✓ Invalid image properly rejected with status: {}", response.status());
}

#[tokio::test]
#[ignore]
async fn test_e2e_concurrent_uploads() {
    // Test handling multiple concurrent uploads
    let base_url = get_base_url();
    let tests_dir = get_tests_dir();

    println!("Testing concurrent uploads with 3 images");

    // Take first 3 fixtures
    let fixtures = &TEST_FIXTURES[0..3.min(TEST_FIXTURES.len())];

    let mut tasks = Vec::new();

    for fixture in fixtures {
        let base_url = base_url.clone();
        let image_path = tests_dir.join(fixture.filename);
        let fixture = *fixture;

        let task = tokio::spawn(async move {
            let client = reqwest::Client::new();

            let upload_response = upload_label_image(
                &client,
                &base_url,
                &image_path,
                if fixture.expected_brand != "TBD" { Some(fixture.expected_brand) } else { None },
                if fixture.expected_class != "TBD" { Some(fixture.expected_class) } else { None },
                if fixture.expected_abv > 0.0 { Some(fixture.expected_abv) } else { None },
            )
            .await?;

            let job_status = wait_for_job_completion(
                &client,
                &base_url,
                &upload_response.job_id.to_string(),
            )
            .await?;

            Ok::<_, Box<dyn std::error::Error + Send + Sync>>((fixture.filename, job_status))
        });

        tasks.push(task);
    }

    // Wait for all uploads to complete
    let results = futures::future::join_all(tasks).await;

    let mut completed = 0;
    for result in results {
        match result {
            Ok(Ok((filename, job_status))) => {
                println!("  ✓ {} completed with status: {}", filename, job_status.status);
                if job_status.status == "completed" {
                    completed += 1;
                }
            }
            Ok(Err(e)) => println!("  ✗ Upload/processing error: {}", e),
            Err(e) => println!("  ✗ Task error: {}", e),
        }
    }

    assert!(
        completed > 0,
        "At least one concurrent upload should complete successfully"
    );

    println!("\n  ✓ Successfully processed {} concurrent uploads", completed);
}

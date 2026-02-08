//! Extract label fields from test images using Workers AI
//!
//! This tool runs OCR on all test images and generates fixture data
//! that can be used in the E2E test suite.
//!
//! Usage: cargo run --example extract_test_labels

use label_verify_hw::services::ocr::WorkersAiClient;
use std::env;
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    println!("# Test Label OCR Extraction\n");

    // Load credentials from environment
    let account_id = env::var("CF_ACCOUNT_ID").expect("CF_ACCOUNT_ID not set");
    let api_token = env::var("CF_API_TOKEN").expect("CF_API_TOKEN not set");

    // Initialize OCR client
    println!("🔌 Connecting to Workers AI...");
    let ocr_client = WorkersAiClient::new(&account_id, &api_token)?;
    println!("✅ Client initialized\n");

    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");

    let mut test_images = Vec::new();
    for entry in fs::read_dir(&tests_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_str().unwrap();
            if filename_str.starts_with("test_label") && filename_str.ends_with(".png") {
                test_images.push(path);
            }
        }
    }

    test_images.sort();

    println!("Found {} test images\n", test_images.len());
    println!("---\n");

    let mut fixtures = Vec::new();

    for (idx, image_path) in test_images.iter().enumerate() {
        let filename = image_path.file_name().unwrap().to_str().unwrap();

        println!("## [{}/{}] Processing: {}", idx + 1, test_images.len(), filename);

        // Get file size
        let metadata = fs::metadata(&image_path)?;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        println!("Size: {:.2} MB", size_mb);

        // Read image
        let image_bytes = fs::read(&image_path)?;

        // Extract fields via OCR
        println!("🔄 Calling Workers AI...");
        match ocr_client.extract_label_fields(&image_bytes).await {
            Ok(extracted) => {
                println!("✅ OCR successful");
                println!("   Brand: {}", extracted.brand_name);
                println!("   Class: {}", extracted.class_type);
                println!("   ABV: {:.1}%", extracted.abv);
                println!("   Net Contents: {}", extracted.net_contents);
                if let Some(country) = &extracted.country_of_origin {
                    println!("   Country: {}", country);
                }

                // Generate fixture template
                println!("\n**Fixture:**");
                println!("```rust");
                println!("TestLabelFixture {{");
                println!("    filename: \"{}\",", filename);
                println!("    expected_brand: \"{}\",", extracted.brand_name);
                println!("    expected_class: \"{}\",", extracted.class_type);
                println!("    expected_abv: {:.1},", extracted.abv);
                println!("    expected_net_contents: \"{}\",", extracted.net_contents);
                println!("    should_pass: true, // Adjust based on validation expectations");
                println!("    description: \"Auto-generated from OCR - {:.2}MB image\",", size_mb);
                println!("}},");
                println!("```");

                fixtures.push((filename.to_string(), extracted, size_mb));
            }
            Err(e) => {
                println!("❌ OCR failed: {}", e);
                println!("\nFixture (failed):");
                println!("```rust");
                println!("TestLabelFixture {{");
                println!("    filename: \"{}\",", filename);
                println!("    expected_brand: \"OCR_FAILED\",");
                println!("    expected_class: \"Unknown\",");
                println!("    expected_abv: 0.0,");
                println!("    expected_net_contents: \"Unknown\",");
                println!("    should_pass: false, // OCR extraction failed");
                println!("    description: \"OCR extraction failed: {}\",", e);
                println!("}},");
                println!("```");
            }
        }

        println!("\n---\n");

        // Add a delay to avoid rate limiting
        if idx < test_images.len() - 1 {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }

    // Generate complete fixtures file
    println!("\n# Complete Fixtures File\n");
    println!("Copy this to `tests/fixtures/mod.rs`:\n");
    println!("```rust");
    println!("//! Test fixtures for E2E testing with real label images");
    println!();
    println!("/// Test fixture representing expected values for a label image");
    println!("#[derive(Debug, Clone)]");
    println!("pub struct TestLabelFixture {{");
    println!("    pub filename: &'static str,");
    println!("    pub expected_brand: &'static str,");
    println!("    pub expected_class: &'static str,");
    println!("    pub expected_abv: f64,");
    println!("    pub expected_net_contents: &'static str,");
    println!("    pub should_pass: bool,");
    println!("    pub description: &'static str,");
    println!("}}");
    println!();
    println!("pub const TEST_FIXTURES: &[TestLabelFixture] = &[");

    for (filename, extracted, size_mb) in &fixtures {
        println!("    TestLabelFixture {{");
        println!("        filename: \"{}\",", filename);
        println!("        expected_brand: \"{}\",", extracted.brand_name);
        println!("        expected_class: \"{}\",", extracted.class_type);
        println!("        expected_abv: {:.1},", extracted.abv);
        println!("        expected_net_contents: \"{}\",", extracted.net_contents);
        println!("        should_pass: true,");
        println!("        description: \"OCR-extracted - {:.2}MB\",", size_mb);
        println!("    }},");
    }

    println!("];");
    println!("```\n");

    println!("\n# Next Steps\n");
    println!("1. Copy the generated fixtures above to `tests/fixtures/mod.rs`");
    println!("2. Review and adjust `should_pass` values based on expected validation results");
    println!("3. Query TTB COLA database for matching records:");
    for (_, extracted, _) in &fixtures {
        println!("   python3 scripts/query_ttb_cola.py --brand \"{}\" --cache", extracted.brand_name);
    }
    println!("4. Run E2E tests: cargo test --test e2e_test -- --ignored");

    println!("\n✅ OCR extraction complete! Processed {} images", fixtures.len());

    Ok(())
}

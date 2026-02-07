//! Example: Test R2 Storage Connection
//!
//! This example verifies that your R2 credentials are configured correctly
//! by uploading and downloading a test file.
//!
//! Usage:
//!   cargo run --example test_r2
//!
//! Prerequisites:
//!   - .env file with R2 credentials (R2_BUCKET, R2_ENDPOINT, R2_ACCESS_KEY, R2_SECRET_KEY)

use label_verify_hw::services::storage::R2Client;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    println!("🔧 R2 Connection Test\n");

    // Load credentials from environment
    let bucket = env::var("R2_BUCKET").expect("R2_BUCKET not set");
    let endpoint = env::var("R2_ENDPOINT").expect("R2_ENDPOINT not set");
    let access_key = env::var("R2_ACCESS_KEY").expect("R2_ACCESS_KEY not set");
    let secret_key = env::var("R2_SECRET_KEY").expect("R2_SECRET_KEY not set");

    println!("📋 Configuration:");
    println!("   Bucket: {}", bucket);
    println!("   Endpoint: {}", endpoint);
    println!("   Access Key: {}***", &access_key[..8.min(access_key.len())]);
    println!();

    // Initialize R2 client
    println!("🔌 Connecting to R2...");
    let client = R2Client::new(&bucket, &endpoint, &access_key, &secret_key)?;
    println!("✅ Client initialized\n");

    // Test upload
    let test_key = "test/connection-test.txt";
    let test_content = b"Hello from label-verify-hw! This is a test file.";
    let content_type = "text/plain";

    println!("⬆️  Uploading test file...");
    println!("   Key: {}", test_key);
    println!("   Size: {} bytes", test_content.len());
    client.upload(test_key, test_content, content_type).await?;
    println!("✅ Upload successful\n");

    // Test download
    println!("⬇️  Downloading test file...");
    let downloaded = client.download(test_key).await?;
    println!("✅ Download successful");
    println!("   Size: {} bytes", downloaded.len());
    println!("   Content: {}", String::from_utf8_lossy(&downloaded));
    println!();

    // Verify content matches
    if downloaded == test_content {
        println!("✅ Content verification passed\n");
    } else {
        println!("❌ Content mismatch!");
        return Err("Downloaded content doesn't match uploaded content".into());
    }

    // Test delete
    println!("🗑️  Deleting test file...");
    client.delete(test_key).await?;
    println!("✅ Delete successful\n");

    // Verify deletion
    println!("🔍 Verifying deletion...");
    match client.download(test_key).await {
        Err(_) => println!("✅ File successfully deleted\n"),
        Ok(_) => {
            println!("❌ File still exists after deletion!");
            return Err("File not deleted properly".into());
        }
    }

    println!("🎉 All R2 tests passed!");
    println!("\n✨ Your R2 configuration is working correctly.");

    Ok(())
}

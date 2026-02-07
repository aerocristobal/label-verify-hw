//! Example: Test Workers AI Connection
//!
//! This example verifies that your Workers AI credentials are configured correctly
//! by making a simple inference request to the LLaVA 1.5 7B model.
//!
//! Usage:
//!   cargo run --example test_workers_ai
//!
//! Prerequisites:
//!   - .env file with CF_ACCOUNT_ID and CF_API_TOKEN

use label_verify_hw::services::ocr::WorkersAIClient;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    println!("🤖 Workers AI Connection Test\n");

    // Load credentials from environment
    let account_id = env::var("CF_ACCOUNT_ID").expect("CF_ACCOUNT_ID not set");
    let api_token = env::var("CF_API_TOKEN").expect("CF_API_TOKEN not set");

    println!("📋 Configuration:");
    println!("   Account ID: {}***", &account_id[..8.min(account_id.len())]);
    println!("   API Token: {}***", &api_token[..12.min(api_token.len())]);
    println!();

    // Initialize Workers AI client
    println!("🔌 Connecting to Workers AI...");
    let client = WorkersAIClient::new(&account_id, &api_token)?;
    println!("✅ Client initialized\n");

    // Create a minimal test image (1x1 red pixel PNG, base64-encoded)
    // This is a valid PNG but won't produce meaningful OCR results - just tests connectivity
    let test_image_base64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";

    println!("🖼️  Testing with minimal image (1x1 pixel)");
    println!("   Note: This tests API connectivity, not OCR accuracy\n");

    // Test API call
    println!("🔄 Sending inference request...");
    println!("   Model: @cf/llava-hf/llava-1.5-7b-hf");
    println!("   Prompt: Describe this image");

    match client.extract_fields(test_image_base64).await {
        Ok(result) => {
            println!("✅ API call successful\n");
            println!("📊 Response:");
            println!("   Brand: {:?}", result.brand_name);
            println!("   Class: {:?}", result.beverage_class);
            println!("   Type: {:?}", result.type_designation);
            println!("   ABV: {:?}", result.alcohol_content);
            println!("   Volume: {:?}", result.net_contents);
            println!("   Origin: {:?}", result.origin);
            println!();
            println!("✨ Workers AI is responding correctly!");
            println!("\n⚠️  Note: The actual OCR results may be inaccurate for this test image.");
            println!("   To test OCR accuracy, use a real beverage label image.");
        }
        Err(e) => {
            println!("❌ API call failed: {}", e);
            println!("\n🔍 Troubleshooting:");
            println!("   1. Verify CF_ACCOUNT_ID is correct");
            println!("   2. Verify CF_API_TOKEN has Workers AI → Read permission");
            println!("   3. Check token hasn't expired");
            println!("   4. Ensure account has Workers AI enabled");
            return Err(e);
        }
    }

    println!("\n🎉 All Workers AI tests passed!");
    println!("\n✅ Your Cloudflare Workers AI configuration is working correctly.");

    Ok(())
}

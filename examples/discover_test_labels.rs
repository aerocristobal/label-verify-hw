//! Helper script to inspect test label images and generate fixture data
//!
//! Usage: cargo run --example discover_test_labels

use image::GenericImageView;
use std::fs;
use std::path::PathBuf;

fn main() {
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");

    println!("# Test Label Image Discovery\n");
    println!("Scanning: {}\n", tests_dir.display());

    let mut images = Vec::new();

    for entry in fs::read_dir(&tests_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_str().unwrap();
            if filename_str.starts_with("test_label") && filename_str.ends_with(".png") {
                images.push(path);
            }
        }
    }

    images.sort();

    for (idx, image_path) in images.iter().enumerate() {
        let filename = image_path.file_name().unwrap().to_str().unwrap();
        let metadata = fs::metadata(&image_path).unwrap();
        let size_kb = metadata.len() / 1024;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

        println!("## {} (Image #{})", filename, idx + 1);
        println!("Size: {:.2} MB ({} KB)", size_mb, size_kb);

        // Try to load image and get dimensions
        match image::open(&image_path) {
            Ok(img) => {
                let (width, height) = img.dimensions();
                println!("Dimensions: {}x{} pixels", width, height);
                println!("Format: {:?}", img.color());
            }
            Err(e) => {
                println!("Error loading image: {}", e);
            }
        }

        println!("\nFixture template:");
        println!("```rust");
        println!("TestLabelFixture {{");
        println!("    filename: \"{}\",", filename);
        println!("    expected_brand: \"TODO\",");
        println!("    expected_class: \"TODO\",");
        println!("    expected_abv: 0.0, // TODO");
        println!("    expected_net_contents: \"TODO\",");
        println!("    should_pass: true, // TODO");
        println!("    description: \"TODO - describe what this image tests\",");
        println!("}},");
        println!("```\n");
        println!("---\n");
    }

    println!("\nTotal test images found: {}", images.len());
    println!("\nNext steps:");
    println!("1. Run: cargo run --example extract_test_labels");
    println!("2. Copy generated fixtures to tests/fixtures/mod.rs");
    println!("3. Query TTB COLA database for matching records");
    println!("4. Update fixture descriptions based on test purpose");
}

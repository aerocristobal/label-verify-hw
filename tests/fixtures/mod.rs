//! Test fixtures for E2E testing with real label images

/// Test fixture representing expected values for a label image
#[derive(Debug, Clone)]
pub struct TestLabelFixture {
    pub filename: &'static str,
    pub expected_brand: &'static str,
    pub expected_class: &'static str,
    pub expected_abv: f64,
    pub expected_net_contents: &'static str,
    pub should_pass: bool,
    pub description: &'static str,
}

/// Test fixtures - to be populated after running OCR extraction
///
/// Run `scripts/extract_test_labels.sh` to generate OCR results,
/// then update these fixtures with actual extracted values.
pub const TEST_FIXTURES: &[TestLabelFixture] = &[
    TestLabelFixture {
        filename: "test_label1.png",
        expected_brand: "TBD",  // Populated after OCR extraction
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 1 - 1.7MB, 580x1450px",
    },
    TestLabelFixture {
        filename: "test_label2.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 2 - 1.5MB, 558x1924px",
    },
    TestLabelFixture {
        filename: "test_label3.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 3 - 2.1MB, 618x2034px",
    },
    TestLabelFixture {
        filename: "test_label4.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 4 - 4.5MB, 998x2580px (largest)",
    },
    TestLabelFixture {
        filename: "test_label5.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 5 - 3.8MB, 1116x2858px",
    },
    TestLabelFixture {
        filename: "test_label6.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 6 - 3.9MB, 946x2762px",
    },
    TestLabelFixture {
        filename: "test_label7.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 7 - 980K, 496x1688px",
    },
    TestLabelFixture {
        filename: "test_label8.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 8 - 871K, 480x1746px",
    },
    TestLabelFixture {
        filename: "test_label9.png",
        expected_brand: "TBD",
        expected_class: "TBD",
        expected_abv: 0.0,
        expected_net_contents: "TBD",
        should_pass: true,
        description: "Test image 9 - 789K, 514x1276px (smallest)",
    },
];

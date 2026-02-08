#!/bin/bash
# Helper script to inspect test label images

echo "# Test Label Image Discovery"
echo ""
echo "Scanning: tests/"
echo ""

count=0
for img in tests/test_label*.png; do
    if [ -f "$img" ]; then
        count=$((count + 1))
        filename=$(basename "$img")
        size=$(ls -lh "$img" | awk '{print $5}')

        echo "## $filename (Image #$count)"
        echo "Size: $size"

        # Get image dimensions using sips (macOS tool)
        if command -v sips &> /dev/null; then
            dims=$(sips -g pixelWidth -g pixelHeight "$img" 2>/dev/null | grep -E "pixelWidth|pixelHeight" | awk '{print $2}' | tr '\n' 'x' | sed 's/x$//')
            echo "Dimensions: ${dims} pixels"
        fi

        echo ""
        echo "Fixture template:"
        echo '```rust'
        echo "TestLabelFixture {"
        echo "    filename: \"$filename\","
        echo "    expected_brand: \"TODO\","
        echo "    expected_class: \"TODO\","
        echo "    expected_abv: 0.0, // TODO"
        echo "    expected_net_contents: \"TODO\","
        echo "    should_pass: true, // TODO"
        echo "    description: \"TODO - describe what this image tests\","
        echo "},"
        echo '```'
        echo ""
        echo "---"
        echo ""
    fi
done

echo ""
echo "Total test images found: $count"
echo ""
echo "Next steps:"
echo "1. Extract OCR results from each image"
echo "2. Query TTB COLA database for matching records"
echo "3. Create test fixtures with expected values"
echo "4. Build E2E test suite"

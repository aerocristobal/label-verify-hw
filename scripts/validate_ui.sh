#!/bin/bash
# UI Implementation Validation Script
# Validates that all required UI improvements are present in static/index.html

set -e

HTML_FILE="static/index.html"
ERRORS=0

echo "🔍 Validating UI Implementation..."
echo "=================================="
echo

# Function to check if pattern exists in file
check_pattern() {
    local pattern="$1"
    local description="$2"

    if grep -q "$pattern" "$HTML_FILE"; then
        echo "✅ $description"
        return 0
    else
        echo "❌ $description"
        ((ERRORS++))
        return 1
    fi
}

# Function to count occurrences
count_pattern() {
    local pattern="$1"
    local expected="$2"
    local description="$3"

    local count=$(grep -c "$pattern" "$HTML_FILE" || true)

    if [ "$count" -ge "$expected" ]; then
        echo "✅ $description (found: $count, expected: ≥$expected)"
        return 0
    else
        echo "❌ $description (found: $count, expected: ≥$expected)"
        ((ERRORS++))
        return 1
    fi
}

echo "📋 Checking Table Structure..."
echo "-------------------------------"
check_pattern "Validation Type" "Table header 'Validation Type' present"
check_pattern "Compliance" "Table header 'Compliance' present"
check_pattern "Details" "Table header 'Details' present"
echo

echo "📊 Checking Compliance Legend..."
echo "--------------------------------"
check_pattern "compliance-legend" "Compliance legend container present"
check_pattern "Compliance Status:" "Legend heading present"
check_pattern "Compliant - Meets TTB requirements" "Compliant legend item present"
check_pattern "Non-Compliant - Fails TTB requirements" "Non-Compliant legend item present"
check_pattern "Warning - Unusual but valid" "Warning legend item present"
check_pattern "Informational check" "Info legend item present"
echo

echo "🎨 Checking CSS Classes..."
echo "--------------------------"
check_pattern "\.compliance-legend" "CSS: .compliance-legend defined"
check_pattern "\.validation-type" "CSS: .validation-type defined"
check_pattern "\.details-btn" "CSS: .details-btn defined"
check_pattern "\.detail-row" "CSS: .detail-row defined"
check_pattern "\.detail-panel" "CSS: .detail-panel defined"
check_pattern "\.comparison-row" "CSS: .comparison-row defined"
check_pattern "\.compliance-box" "CSS: .compliance-box defined"
check_pattern "\.status-compliant" "CSS: .status-compliant defined"
check_pattern "\.status-non-compliant" "CSS: .status-non-compliant defined"
check_pattern "\.status-warning" "CSS: .status-warning defined"
check_pattern "\.status-info" "CSS: .status-info defined"
check_pattern "\.row-compliant" "CSS: .row-compliant defined"
check_pattern "\.row-non-compliant" "CSS: .row-non-compliant defined"
check_pattern "\.row-warning" "CSS: .row-warning defined"
check_pattern "\.row-info" "CSS: .row-info defined"
echo

echo "⚙️ Checking JavaScript Functions..."
echo "------------------------------------"
check_pattern "function displayResultSummary" "JS: displayResultSummary function"
check_pattern "function renderFieldResults" "JS: renderFieldResults function"
check_pattern "function formatFieldName" "JS: formatFieldName function"
check_pattern "function getRowClass" "JS: getRowClass function"
check_pattern "function getValidationType" "JS: getValidationType function"
check_pattern "function getResultDisplay" "JS: getResultDisplay function"
check_pattern "function getComplianceStatus" "JS: getComplianceStatus function"
check_pattern "function createDetailRow" "JS: createDetailRow function"
check_pattern "function generateDetailPanel" "JS: generateDetailPanel function"
check_pattern "function formatMatchType" "JS: formatMatchType function"
check_pattern "function toggleDetails" "JS: toggleDetails function"
echo

echo "🏷️ Checking Validation Type Icons..."
echo "-------------------------------------"
check_pattern "📝 User Input" "User Input validation type"
check_pattern "🗄️ TTB Database" "TTB Database validation type"
check_pattern "📜 TTB Regulation" "TTB Regulation validation type"
check_pattern "ℹ️ Format Check" "Format Check validation type"
echo

echo "✨ Checking Status Icons..."
echo "---------------------------"
count_pattern "✓" 5 "Compliant checkmark icons (✓)"
count_pattern "✗" 3 "Non-compliant cross icons (✗)"
count_pattern "⚠" 2 "Warning icons (⚠)"
count_pattern "ℹ" 2 "Info icons (ℹ)"
echo

echo "🎯 Checking Detail Panel Components..."
echo "---------------------------------------"
check_pattern "Comparison Details" "Comparison Details section heading"
check_pattern "Compliance Determination" "Compliance Determination section heading"
check_pattern "Your Input:" "User input comparison label"
check_pattern "Found on Label:" "Found value comparison label"
check_pattern "Database Record:" "Database record label"
check_pattern "Record ID:" "Record ID label"
check_pattern "Regulation:" "Regulation label"
check_pattern "Match Type:" "Match Type label"
check_pattern "Confidence:" "Confidence label"
echo

echo "📱 Checking Responsive Design..."
echo "--------------------------------"
check_pattern "@media (max-width: 600px)" "Mobile media query present"
check_pattern "grid-template-columns: 1fr" "Mobile single-column layout"
echo

echo "♿ Checking Accessibility..."
echo "---------------------------"
check_pattern "aria-label" "ARIA labels present"
check_pattern "Show details" "ARIA label for show details"
check_pattern "Hide details" "ARIA label for hide details"
echo

echo "=================================="
echo

if [ $ERRORS -eq 0 ]; then
    echo "✅ All validation checks passed!"
    echo
    echo "Next steps:"
    echo "1. Start the server: cargo run"
    echo "2. Open http://localhost:3000 in browser"
    echo "3. Submit a test label to verify UI changes"
    echo "4. Click [+] buttons to test expandable details"
    echo "5. Test on mobile device or resize browser window"
    exit 0
else
    echo "❌ Validation failed with $ERRORS error(s)"
    echo
    echo "Please review the errors above and ensure all"
    echo "required UI components are properly implemented."
    exit 1
fi

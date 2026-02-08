//! TTB COLA Public Registry Client
//!
//! Queries the TTB (Alcohol and Tobacco Tax and Trade Bureau) COLA (Certificate of Label Approval)
//! public database to retrieve approved beverage labels.
//!
//! Official Source: <https://ttbonline.gov/colasonline/publicSearchColasBasic.do>

use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Global TTB COLA client (lazily initialized).
static TTB_CLIENT: OnceLock<TtbColaClient> = OnceLock::new();

/// Get or initialize the global TTB COLA client.
pub fn get_client() -> Result<&'static TtbColaClient, TtbColaError> {
    if let Some(client) = TTB_CLIENT.get() {
        return Ok(client);
    }
    let client = TtbColaClient::new()?;
    // Another thread may have initialized between our get() and set().
    // OnceLock::set returns Err with the value if already initialized.
    let _ = TTB_CLIENT.set(client);
    Ok(TTB_CLIENT.get().unwrap())
}

/// A single COLA record from the TTB public database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtbColaRecord {
    pub ttb_id: String,
    pub permit_no: String,
    pub serial_number: String,
    pub completed_date: Option<NaiveDate>,
    pub fanciful_name: Option<String>,
    pub brand_name: String,
    pub origin_code: String,
    pub origin_desc: String,
    pub class_type_code: String,
    pub class_type_desc: String,
    pub source_url: String,
    pub inferred_abv: Option<f64>,
    pub beverage_category: String,
}

/// Error type for TTB COLA client operations.
#[derive(Debug, thiserror::Error)]
pub enum TtbColaError {
    #[error("HTTP request to TTB failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Failed to parse TTB response HTML: {0}")]
    Parse(String),

    #[error("TTB service unavailable: {0}")]
    Unavailable(String),
}

/// Client for querying the TTB COLA public database.
pub struct TtbColaClient {
    http: reqwest::Client,
    base_url: String,
}

impl TtbColaClient {
    /// Create a new TTB COLA client.
    ///
    /// Uses `danger_accept_invalid_certs(true)` because the TTB website
    /// has recurring SSL certificate issues.
    pub fn new() -> Result<Self, TtbColaError> {
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent("Mozilla/5.0 (compatible; LabelVerifyBot/1.0; +https://github.com/aerocristobal/label-verify-hw)")
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http,
            base_url: "https://ttbonline.gov/colasonline".to_string(),
        })
    }

    /// Search TTB COLA public database by brand name.
    ///
    /// Uses the TTB search form with `productOrFancifulName` parameter
    /// and a 5-year lookback window for maximum recall.
    pub async fn search_by_brand(
        &self,
        brand_name: &str,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TtbColaRecord>, TtbColaError> {
        let now = chrono::Utc::now();
        let from_date = now - chrono::Duration::days(5 * 365);

        let mut params = vec![
            ("searchCriteria.dateCompletedFrom", from_date.format("%m/%d/%Y").to_string()),
            ("searchCriteria.dateCompletedTo", now.format("%m/%d/%Y").to_string()),
            ("searchCriteria.productOrFancifulName", brand_name.to_string()),
            ("searchCriteria.productNameSearchType", "E".to_string()),
        ];

        // Add category-specific class type code ranges
        if let Some(cat) = category {
            match cat {
                "wine" => {
                    params.push(("searchCriteria.classTypeFrom", "80".to_string()));
                    params.push(("searchCriteria.classTypeTo", "89".to_string()));
                }
                "distilled_spirits" => {
                    params.push(("searchCriteria.classTypeFrom", "100".to_string()));
                    params.push(("searchCriteria.classTypeTo", "699".to_string()));
                }
                "malt_beverage" => {
                    params.push(("searchCriteria.classTypeFrom", "900".to_string()));
                    params.push(("searchCriteria.classTypeTo", "999".to_string()));
                }
                _ => {}
            }
        }

        let url = format!("{}/publicSearchColasBasicProcess.do?action=search", self.base_url);

        let response = self
            .http
            .post(&url)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(TtbColaError::Unavailable(format!(
                "TTB returned HTTP {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        self.parse_search_results(&html, limit)
    }

    /// Parse HTML search results table from TTB COLA response.
    ///
    /// TTB results table structure (10 columns):
    /// TTB ID | Permit No. | Serial Number | Completed Date | Fanciful Name |
    /// Brand Name | Origin Code | Origin Desc | Class/Type Code | Class/Type Desc
    fn parse_search_results(
        &self,
        html: &str,
        limit: usize,
    ) -> Result<Vec<TtbColaRecord>, TtbColaError> {
        // Check for "No results" message
        if html.contains("No results were found") {
            return Ok(Vec::new());
        }

        let document = Html::parse_document(html);
        let table_sel = Selector::parse("table").expect("valid selector");
        let tr_sel = Selector::parse("tr").expect("valid selector");
        let td_sel = Selector::parse("td").expect("valid selector");
        let a_sel = Selector::parse("a").expect("valid selector");

        // Find the results table (contains "TTB ID" and "Brand Name")
        let results_table = document
            .select(&table_sel)
            .find(|table| {
                let text: String = table.text().collect();
                text.contains("TTB ID") && text.contains("Brand Name") && text.contains("Class/Type")
            });

        let results_table = match results_table {
            Some(table) => table,
            None => {
                // No results table found — might be an empty result page
                return Ok(Vec::new());
            }
        };

        let mut records = Vec::new();
        let rows: Vec<_> = results_table.select(&tr_sel).collect();

        // Skip header row (index 0), process data rows
        for row in rows.iter().skip(1) {
            let cells: Vec<_> = row.select(&td_sel).collect();
            if cells.len() < 10 {
                continue;
            }

            let ttb_id = cells[0].text().collect::<String>().trim().to_string();
            let permit_no = cells[1].text().collect::<String>().trim().to_string();
            let serial_number = cells[2].text().collect::<String>().trim().to_string();
            let completed_date_str = cells[3].text().collect::<String>().trim().to_string();
            let fanciful_name_raw = cells[4].text().collect::<String>().trim().to_string();
            let brand_name = cells[5].text().collect::<String>().trim().to_string();
            let origin_code = cells[6].text().collect::<String>().trim().to_string();
            let origin_desc = cells[7].text().collect::<String>().trim().to_string();
            let class_type_code = cells[8].text().collect::<String>().trim().to_string();
            let class_type_desc = cells[9].text().collect::<String>().trim().to_string();

            // Skip if missing critical fields
            if ttb_id.is_empty() || brand_name.is_empty() || class_type_desc.is_empty() {
                continue;
            }

            // Parse completed date (MM/DD/YYYY format)
            let completed_date = NaiveDate::parse_from_str(&completed_date_str, "%m/%d/%Y").ok();

            // Extract detail URL from first cell's <a> tag
            let source_url = cells[0]
                .select(&a_sel)
                .next()
                .and_then(|a| a.value().attr("href"))
                .map(|href| {
                    if href.starts_with("http") {
                        href.to_string()
                    } else {
                        format!("{}/{}", self.base_url, href)
                    }
                })
                .unwrap_or_else(|| {
                    format!(
                        "{}/viewColaDetails.do?action=publicDisplaySearchBasic&ttbid={}",
                        self.base_url, ttb_id
                    )
                });

            let fanciful_name = if fanciful_name_raw.is_empty() {
                None
            } else {
                Some(fanciful_name_raw)
            };

            let inferred_abv = infer_abv_from_class_type(&class_type_desc);
            let beverage_category = get_category_from_class_type(&class_type_desc, &class_type_code);

            records.push(TtbColaRecord {
                ttb_id,
                permit_no,
                serial_number,
                completed_date,
                fanciful_name,
                brand_name,
                origin_code,
                origin_desc,
                class_type_code,
                class_type_desc,
                source_url,
                inferred_abv,
                beverage_category,
            });

            if records.len() >= limit {
                break;
            }
        }

        Ok(records)
    }
}

/// Infer ABV from TTB class/type description using regulatory ranges.
///
/// TTB COLA results do NOT include ABV. We infer typical values
/// based on 27 CFR regulations.
pub fn infer_abv_from_class_type(class_type_desc: &str) -> Option<f64> {
    let normalized = class_type_desc.to_uppercase();

    // Wine keywords (more specific first)
    if normalized.contains("DESSERT")
        || normalized.contains("PORT")
        || normalized.contains("SHERRY")
        || normalized.contains("COOKING")
    {
        return Some(18.0);
    }
    if normalized.contains("TABLE WINE")
        || normalized.contains("WHITE WINE")
        || normalized.contains("RED WINE")
    {
        return Some(12.0);
    }
    if normalized.contains("SPARKLING") || normalized.contains("CHAMPAGNE") {
        return Some(12.0);
    }

    // Spirits keywords
    if normalized.contains("WHISKEY") || normalized.contains("WHISKY") || normalized.contains("BOURBON") {
        return Some(45.0);
    }
    if normalized.contains("GIN") {
        return Some(40.0);
    }
    if normalized.contains("VODKA") {
        return Some(40.0);
    }
    if normalized.contains("RUM") {
        return Some(40.0);
    }
    if normalized.contains("TEQUILA") {
        return Some(40.0);
    }
    if normalized.contains("BRANDY") {
        return Some(40.0);
    }

    // Malt beverage keywords (more specific first)
    if normalized.contains("IPA") || normalized.contains("INDIA PALE ALE") {
        return Some(6.5);
    }
    if normalized.contains("STOUT") || normalized.contains("PORTER") {
        return Some(6.0);
    }
    if normalized.contains("BEER") || normalized.contains("LAGER") || normalized.contains("ALE") {
        return Some(5.0);
    }
    if normalized.contains("MALT BEVERAGE") {
        return Some(5.0);
    }

    // Broad category fallbacks
    if normalized.contains("WINE") {
        return Some(12.0);
    }
    if normalized.contains("SPIRIT") || normalized.contains("LIQUOR") || normalized.contains("LIQUEUR") {
        return Some(40.0);
    }
    if normalized.contains("MALT") {
        return Some(5.0);
    }

    None
}

/// Map TTB class/type to beverage category.
///
/// Uses description keywords first, falls back to numeric code ranges.
pub fn get_category_from_class_type(class_type_desc: &str, class_type_code: &str) -> String {
    let normalized = class_type_desc.to_uppercase();

    // Wine indicators
    if ["WINE", "CHAMPAGNE", "PORT", "SHERRY", "DESSERT", "TABLE"]
        .iter()
        .any(|kw| normalized.contains(kw))
    {
        return "wine".to_string();
    }

    // Spirits indicators
    if [
        "WHISKEY", "WHISKY", "BOURBON", "GIN", "VODKA", "RUM", "TEQUILA", "BRANDY", "LIQUEUR",
        "SPIRIT", "DISTILLED",
    ]
    .iter()
    .any(|kw| normalized.contains(kw))
    {
        return "distilled_spirits".to_string();
    }

    // Malt beverage indicators
    if ["BEER", "ALE", "LAGER", "MALT", "IPA", "STOUT", "PORTER"]
        .iter()
        .any(|kw| normalized.contains(kw))
    {
        return "malt_beverage".to_string();
    }

    // Fallback to numeric code ranges
    if let Ok(code) = class_type_code.parse::<i32>() {
        if (80..=89).contains(&code) {
            return "wine".to_string();
        }
        if (100..=699).contains(&code) {
            return "distilled_spirits".to_string();
        }
        if (900..=999).contains(&code) {
            return "malt_beverage".to_string();
        }
    }

    "wine".to_string() // Default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_abv_wine() {
        assert_eq!(infer_abv_from_class_type("TABLE RED WINE"), Some(12.0));
        assert_eq!(infer_abv_from_class_type("TABLE WHITE WINE"), Some(12.0));
        assert_eq!(infer_abv_from_class_type("DESSERT WINE"), Some(18.0));
        assert_eq!(infer_abv_from_class_type("SPARKLING WINE/CHAMPAGNE"), Some(12.0));
    }

    #[test]
    fn test_infer_abv_spirits() {
        assert_eq!(infer_abv_from_class_type("STRAIGHT BOURBON WHISKY"), Some(45.0));
        assert_eq!(infer_abv_from_class_type("VODKA"), Some(40.0));
        assert_eq!(infer_abv_from_class_type("GIN"), Some(40.0));
        assert_eq!(infer_abv_from_class_type("RUM"), Some(40.0));
        assert_eq!(infer_abv_from_class_type("TEQUILA"), Some(40.0));
    }

    #[test]
    fn test_infer_abv_malt() {
        assert_eq!(infer_abv_from_class_type("BEER"), Some(5.0));
        assert_eq!(infer_abv_from_class_type("IPA"), Some(6.5));
        assert_eq!(infer_abv_from_class_type("STOUT"), Some(6.0));
        assert_eq!(
            infer_abv_from_class_type("MALT BEVERAGES SPECIALITIES - FLAVORED"),
            Some(5.0)
        );
    }

    #[test]
    fn test_infer_abv_unknown() {
        assert_eq!(infer_abv_from_class_type("SOMETHING UNKNOWN"), None);
    }

    #[test]
    fn test_category_from_desc() {
        assert_eq!(get_category_from_class_type("TABLE RED WINE", "80"), "wine");
        assert_eq!(
            get_category_from_class_type("STRAIGHT BOURBON WHISKEY", "170"),
            "distilled_spirits"
        );
        assert_eq!(get_category_from_class_type("BEER", "901"), "malt_beverage");
    }

    #[test]
    fn test_category_from_code_fallback() {
        assert_eq!(get_category_from_class_type("UNKNOWN", "80"), "wine");
        assert_eq!(get_category_from_class_type("UNKNOWN", "85"), "wine");
        assert_eq!(get_category_from_class_type("UNKNOWN", "100"), "distilled_spirits");
        assert_eq!(get_category_from_class_type("UNKNOWN", "500"), "distilled_spirits");
        assert_eq!(get_category_from_class_type("UNKNOWN", "901"), "malt_beverage");
    }

    #[test]
    fn test_category_default() {
        assert_eq!(get_category_from_class_type("UNKNOWN", "0"), "wine");
    }

    #[test]
    fn test_parse_no_results() {
        let client = TtbColaClient::new().unwrap();
        let result = client
            .parse_search_results("No results were found for your search criteria", 10)
            .unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_empty_html() {
        let client = TtbColaClient::new().unwrap();
        let result = client.parse_search_results("<html><body></body></html>", 10).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_valid_table() {
        let client = TtbColaClient::new().unwrap();
        let html = r#"
        <html><body>
        <table>
            <tr><th>TTB ID</th><th>Permit</th><th>Serial</th><th>Date</th><th>Fanciful</th>
                <th>Brand Name</th><th>Origin</th><th>Origin Desc</th><th>Class/Type</th><th>Class/Type Desc</th></tr>
            <tr>
                <td><a href="viewColaDetails.do?ttbid=123">123</a></td>
                <td>BWN-CA-12345</td>
                <td>250001</td>
                <td>01/15/2026</td>
                <td>Reserve</td>
                <td>FETZER</td>
                <td>06</td>
                <td>CALIFORNIA</td>
                <td>80</td>
                <td>TABLE RED WINE</td>
            </tr>
        </table>
        </body></html>
        "#;
        let records = client.parse_search_results(html, 10).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].brand_name, "FETZER");
        assert_eq!(records[0].class_type_desc, "TABLE RED WINE");
        assert_eq!(records[0].inferred_abv, Some(12.0));
        assert_eq!(records[0].beverage_category, "wine");
        assert_eq!(records[0].fanciful_name, Some("Reserve".to_string()));
    }
}

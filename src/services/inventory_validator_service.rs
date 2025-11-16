/// Enterprise-grade inventory validation service
/// Validates data quality, enriches with OpenFDA, enforces business rules

use uuid::Uuid;
use sqlx::PgPool;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use crate::middleware::error_handling::{Result, AppError};
use crate::models::ai_import::{MappedInventoryRow, ColumnMapping};
use crate::repositories::OpenFdaRepository;
use crate::services::OpenFdaService;
use std::str::FromStr;

pub struct InventoryValidatorService {
    db_pool: PgPool,
    openfda_service: OpenFdaService,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub enriched_data: Option<EnrichedData>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EnrichedData {
    pub matched_ndc: Option<String>,
    pub openfda_brand_name: Option<String>,
    pub openfda_generic_name: Option<String>,
    pub openfda_manufacturer: Option<String>,
    pub openfda_dosage_form: Option<String>,
    pub openfda_strength: Option<String>,
    pub confidence_score: f32,
}

impl InventoryValidatorService {
    pub fn new(db_pool: PgPool) -> Self {
        let openfda_repo = OpenFdaRepository::new(db_pool.clone());
        let openfda_service = OpenFdaService::new(openfda_repo);

        Self {
            db_pool,
            openfda_service,
        }
    }

    /// Validate and enrich a single inventory row
    pub async fn validate_row(
        &self,
        row: &MappedInventoryRow,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut enriched_data = None;

        // 1. Validate required fields
        if row.quantity.is_none() || row.quantity.unwrap_or(0) <= 0 {
            errors.push("Quantity is required and must be positive".to_string());
        }

        // 2. Validate NDC format if provided
        if let Some(ref ndc) = row.ndc_code {
            if !Self::is_valid_ndc_format(ndc) {
                warnings.push(format!("NDC '{}' has non-standard format. Expected: 5-4-2 (e.g., 12345-678-90)", ndc));
            }

            // Try to enrich from OpenFDA
            match self.openfda_service.get_by_ndc(ndc).await {
                Ok(Some(fda_drug)) => {
                    let confidence = Self::calculate_match_confidence(row, &fda_drug);
                    
                    enriched_data = Some(EnrichedData {
                        matched_ndc: Some(fda_drug.product_ndc.clone()),
                        openfda_brand_name: Some(fda_drug.brand_name.clone()),
                        openfda_generic_name: Some(fda_drug.generic_name.clone()),
                        openfda_manufacturer: Some(fda_drug.labeler_name.clone()),
                        openfda_dosage_form: fda_drug.dosage_form.clone(),
                        openfda_strength: fda_drug.strength.clone(),
                        confidence_score: confidence,
                    });

                    if confidence >= 0.9 {
                        tracing::info!("High-confidence OpenFDA match for NDC: {}", ndc);
                    } else if confidence >= 0.7 {
                        warnings.push(format!("Moderate OpenFDA match confidence ({:.0}%) for NDC: {}", confidence * 100.0, ndc));
                    } else {
                        warnings.push(format!("Low OpenFDA match confidence ({:.0}%) for NDC: {}", confidence * 100.0, ndc));
                    }
                }
                Ok(None) => {
                    warnings.push(format!("NDC '{}' not found in FDA catalog. Please verify.", ndc));
                }
                Err(e) => {
                    tracing::warn!("Failed to lookup NDC {}: {}", ndc, e);
                    warnings.push("Unable to verify against FDA catalog".to_string());
                }
            }
        } else {
            warnings.push("No NDC code provided - cannot verify against FDA catalog".to_string());
        }

        // 3. Validate product names
        if row.brand_name.is_none() && row.generic_name.is_none() {
            errors.push("Either brand name or generic name is required".to_string());
        }

        // 4. Validate expiry date
        if let Some(expiry) = row.expiry_date {
            if expiry < chrono::Utc::now().date_naive() {
                errors.push(format!("Product has expired (expiry: {})", expiry));
            } else if expiry < chrono::Utc::now().date_naive() + chrono::Duration::days(90) {
                warnings.push(format!("Product expires soon (expiry: {})", expiry));
            }
        } else {
            warnings.push("No expiry date provided".to_string());
        }

        // 5. Validate price
        if let Some(price) = row.unit_price {
            if price <= Decimal::ZERO {
                errors.push("Unit price must be positive".to_string());
            } else if price > Decimal::from(10000) {
                warnings.push(format!("Unusually high unit price: ${}", price));
            }
        }

        // 6. Validate batch number
        if row.batch_number.is_none() || row.batch_number.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
            warnings.push("No batch/lot number provided".to_string());
        }

        // 7. Business logic validations
        if let (Some(quantity), Some(price)) = (row.quantity, row.unit_price) {
            let total_value = Decimal::from(quantity) * price;
            if total_value > Decimal::from(1_000_000) {
                warnings.push(format!("High-value inventory item: ${} total", total_value));
            }
        }

        let is_valid = errors.is_empty();

        Ok(ValidationResult {
            is_valid,
            errors,
            warnings,
            enriched_data,
        })
    }

    /// Validate NDC format (5-4-2 or variants)
    fn is_valid_ndc_format(ndc: &str) -> bool {
        // Standard format: 5-4-2 (e.g., 12345-678-90)
        // Also accept: 4-4-2, 5-3-2 variants
        let parts: Vec<&str> = ndc.split('-').collect();
        
        if parts.len() != 3 {
            return false;
        }

        // Check each part is numeric
        parts.iter().all(|part| part.chars().all(|c| c.is_numeric()) && !part.is_empty())
    }

    /// Calculate confidence score for OpenFDA match
    fn calculate_match_confidence(
        row: &MappedInventoryRow,
        fda_drug: &crate::models::openfda::OpenFdaCatalogResponse,
    ) -> f32 {
        let mut score = 1.0f32; // Start with perfect match
        let mut checks = 0;

        // Compare brand names if available
        if let Some(ref brand_name) = row.brand_name {
            checks += 1;
            if !Self::fuzzy_match(&fda_drug.brand_name, brand_name) {
                score -= 0.3;
            }
        }

        // Compare generic names if available
        if let Some(ref generic_name) = row.generic_name {
            checks += 1;
            if !Self::fuzzy_match(&fda_drug.generic_name, generic_name) {
                score -= 0.3;
            }
        }

        // Compare manufacturer if available
        if let Some(ref manufacturer) = row.manufacturer {
            checks += 1;
            if !Self::fuzzy_match(&fda_drug.labeler_name, manufacturer) {
                score -= 0.2;
            }
        }

        // If no additional fields to check, lower confidence
        if checks == 0 {
            score -= 0.2;
        }

        score.max(0.0).min(1.0)
    }

    /// Fuzzy string matching for drug names (case-insensitive, trim whitespace)
    fn fuzzy_match(s1: &str, s2: &str) -> bool {
        let s1_normalized = s1.trim().to_lowercase();
        let s2_normalized = s2.trim().to_lowercase();

        // Exact match
        if s1_normalized == s2_normalized {
            return true;
        }

        // One contains the other
        if s1_normalized.contains(&s2_normalized) || s2_normalized.contains(&s1_normalized) {
            return true;
        }

        // Calculate similarity (simple approach)
        let similarity = Self::calculate_similarity(&s1_normalized, &s2_normalized);
        similarity > 0.8
    }

    /// Calculate string similarity (Jaro-Winkler-like)
    fn calculate_similarity(s1: &str, s2: &str) -> f32 {
        if s1 == s2 {
            return 1.0;
        }

        let s1_len = s1.len();
        let s2_len = s2.len();

        if s1_len == 0 || s2_len == 0 {
            return 0.0;
        }

        // Simple character-based similarity
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        let mut matches = 0;
        let max_len = s1_len.max(s2_len);

        for i in 0..s1_len.min(s2_len) {
            if s1_chars[i] == s2_chars[i] {
                matches += 1;
            }
        }

        matches as f32 / max_len as f32
    }

    /// Parse and validate data from raw row using column mapping
    pub fn map_row_to_inventory(
        &self,
        row_number: usize,
        headers: &[String],
        row_data: &[String],
        mapping: &ColumnMapping,
    ) -> Result<MappedInventoryRow> {
        let mut mapped = MappedInventoryRow {
            row_number,
            ndc_code: None,
            brand_name: None,
            generic_name: None,
            manufacturer: None,
            quantity: None,
            batch_number: None,
            expiry_date: None,
            unit_price: None,
            storage_location: None,
            category: None,
            strength: None,
            dosage_form: None,
            validation_errors: Vec::new(),
            validation_warnings: Vec::new(),
        };

        // Helper to get value from row by column name
        let get_value = |col_name: &Option<String>| -> Option<String> {
            col_name.as_ref().and_then(|name| {
                headers.iter()
                    .position(|h| h == name)
                    .and_then(|idx| row_data.get(idx))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
        };

        // Map fields
        mapped.ndc_code = get_value(&mapping.ndc_code);
        mapped.brand_name = get_value(&mapping.brand_name);
        mapped.generic_name = get_value(&mapping.generic_name);
        mapped.manufacturer = get_value(&mapping.manufacturer);
        mapped.batch_number = get_value(&mapping.batch_number);
        mapped.storage_location = get_value(&mapping.storage_location);
        mapped.category = get_value(&mapping.category);
        mapped.strength = get_value(&mapping.strength);
        mapped.dosage_form = get_value(&mapping.dosage_form);

        // Parse quantity
        if let Some(qty_str) = get_value(&mapping.quantity) {
            match qty_str.parse::<i32>() {
                Ok(qty) => mapped.quantity = Some(qty),
                Err(_) => {
                    mapped.validation_errors.push(format!("Invalid quantity: '{}'", qty_str));
                }
            }
        }

        // Parse unit price
        if let Some(price_str) = get_value(&mapping.unit_price) {
            // Remove currency symbols and commas
            let cleaned = price_str.replace("$", "").replace(",", "").trim().to_string();
            match Decimal::from_str(&cleaned) {
                Ok(price) => mapped.unit_price = Some(price),
                Err(_) => {
                    mapped.validation_errors.push(format!("Invalid price: '{}'", price_str));
                }
            }
        }

        // Parse expiry date
        if let Some(date_str) = get_value(&mapping.expiry_date) {
            match Self::parse_flexible_date(&date_str) {
                Some(date) => mapped.expiry_date = Some(date),
                None => {
                    mapped.validation_errors.push(format!("Invalid date format: '{}'", date_str));
                }
            }
        }

        Ok(mapped)
    }

    /// Parse dates in various formats
    fn parse_flexible_date(date_str: &str) -> Option<NaiveDate> {
        let formats = [
            "%Y-%m-%d",       // 2025-12-31
            "%m/%d/%Y",       // 12/31/2025
            "%d-%m-%Y",       // 31-12-2025
            "%Y/%m/%d",       // 2025/12/31
            "%d/%m/%Y",       // 31/12/2025
            "%B %d, %Y",      // December 31, 2025
            "%b %d, %Y",      // Dec 31, 2025
            "%Y%m%d",         // 20251231
        ];

        for format in &formats {
            if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
                return Some(date);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ndc_validation() {
        assert!(InventoryValidatorService::is_valid_ndc_format("12345-678-90"));
        assert!(InventoryValidatorService::is_valid_ndc_format("1234-5678-90"));
        assert!(!InventoryValidatorService::is_valid_ndc_format("12345-678"));
        assert!(!InventoryValidatorService::is_valid_ndc_format("invalid"));
    }

    #[test]
    fn test_date_parsing() {
        assert!(InventoryValidatorService::parse_flexible_date("2025-12-31").is_some());
        assert!(InventoryValidatorService::parse_flexible_date("12/31/2025").is_some());
        assert!(InventoryValidatorService::parse_flexible_date("31-12-2025").is_some());
        assert!(InventoryValidatorService::parse_flexible_date("20251231").is_some());
        assert!(InventoryValidatorService::parse_flexible_date("invalid").is_none());
    }

    #[test]
    fn test_fuzzy_matching() {
        assert!(InventoryValidatorService::fuzzy_match("Amoxicillin", "amoxicillin"));
        assert!(InventoryValidatorService::fuzzy_match("Amoxicillin 500mg", "Amoxicillin"));
        assert!(!InventoryValidatorService::fuzzy_match("Amoxicillin", "Ibuprofen"));
    }
}

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// EMA ePI API Response Models (FHIR Bundle Structure)
// ============================================================================

/// EMA ePI API Bundle response following FHIR Bundle structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EmaEpiApiResponse {
    #[serde(rename = "resourceType")]
    pub resource_type: String,  // Should be "Bundle"
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub bundle_type: Option<String>,  // "searchset", "collection", etc.
    pub link: Option<Vec<BundleLink>>,
    pub entry: Option<Vec<EpiEntry>>,
    pub total: Option<i32>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BundleLink {
    pub relation: Option<String>,
    pub url: Option<String>,
}

/// Individual entry in EPI Bundle
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiEntry {
    pub full_url: Option<String>,  // Canonical URL for the resource
    pub resource: EpiResource,
    pub search: Option<EpiSearch>,
}

/// Search information within Bundle entry
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiSearch {
    pub mode: Option<String>,  // "match" | "include" | "outcome"
    pub score: Option<f64>,    // Search relevance score
}

/// Main EPI resource containing regulatory information
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiResource {
    #[serde(rename = "resourceType")]
    pub resource_type: String,  // "MedicinalProductDefinition", "BundledProduct", etc.
    pub id: Option<String>,
    pub identifier: Option<Vec<EpiIdentifier>>,
    pub meta: Option<EpiMeta>,

    // Product identification
    pub title: Option<EpiCodeableConcept>,
    pub name: Option<Vec<EpiCodeableConcept>>,
    pub code: Option<EpiCodeableConcept>,

    // Authorization information
    pub status: Option<String>,  // "active" | "inactive" | "entered-in-error"
    pub date: Option<String>,    // Publication date
    pub author: Option<Vec<EpiReference>>,
    pub publisher: Option<String>,
    pub contact: Option<Vec<EpiContact>>,

    // Classification
    pub subject: Option<EpiReference>,
    pub domain: Option<EpiCodeableConcept>,
    pub jurisdiction: Option<Vec<EpiCodeableConcept>>,

    // Product details
    pub description: Option<String>,
    pub purpose: Option<String>,
    pub usage: Option<String>,
    pub approval_date: Option<String>,
    pub last_review_date: Option<String>,

    // Language and localization
    pub language: Option<String>,
    pub contained: Option<Vec<serde_json::Value>>,
    pub extension: Option<Vec<EpiExtension>>,

    // Capture any additional fields
    #[serde(flatten)]
    pub additional_fields: std::collections::HashMap<String, serde_json::Value>,
}

/// EPI identifier structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiIdentifier {
    pub system: Option<String>,  // URL identifying the system
    pub value: Option<String>,   // The actual value
    #[serde(rename = "type")]
    pub identifier_type: Option<EpiCodeableConcept>,
    pub period: Option<EpiPeriod>,
    pub assigner: Option<Box<EpiReference>>,  // Box to break circular dependency
}

/// Codeable concept for coded data
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum EpiCodeableConcept {
    Simple {
        coding: Option<Vec<EpiCoding>>,
        text: Option<String>,
    },
    Text(String),
}

/// Coding structure with system/code/display
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiCoding {
    pub system: Option<String>,   // URL for the coding system
    pub version: Option<String>,
    pub code: Option<String>,
    pub display: Option<String>,
    pub user_selected: Option<bool>,
}

/// Reference to another resource
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiReference {
    pub reference: Option<String>,    // Reference string (e.g., "Resource/123")
    #[serde(rename = "type")]
    pub reference_type: Option<String>,
    pub identifier: Option<Box<EpiIdentifier>>,  // Box to break circular dependency
    pub display: Option<String>,      // Human-readable display
}

/// Contact information
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiContact {
    pub name: Option<EpiHumanName>,
    pub telecom: Option<Vec<EpiContactPoint>>,
    pub address: Option<EpiAddress>,
}

/// Human name structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiHumanName {
    #[serde(rename = "use")]
    pub r#use: Option<String>,      // "usual" | "official" | "temp" | "nickname" | "anonymous" | "old" | "maiden"
    pub text: Option<String>,     // Full text representation
    pub family: Option<String>,   // Family name
    pub given: Option<Vec<String>>, // Given names
    pub prefix: Option<Vec<String>>,
    pub suffix: Option<Vec<String>>,
    pub period: Option<EpiPeriod>,
}

/// Contact point (phone, email, etc.)
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiContactPoint {
    pub system: Option<String>,   // "phone" | "fax" | "email" | "pager" | "url" | "sms" | "other"
    pub value: Option<String>,
    #[serde(rename = "use")]
    pub r#use: Option<String>,      // "home" | "work" | "temp" | "old" | "mobile"
    pub rank: Option<i32>,
    pub period: Option<EpiPeriod>,
}

/// Address structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiAddress {
    #[serde(rename = "use")]
    pub r#use: Option<String>,      // "home" | "work" | "temp" | "old"
    #[serde(rename = "type")]
    pub address_type: Option<String>, // "postal" | "physical" | "both"
    pub text: Option<String>,     // Full address
    pub line: Option<Vec<String>>, // Street lines
    pub city: Option<String>,
    pub district: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub period: Option<EpiPeriod>,
}

/// Time period
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiPeriod {
    pub start: Option<String>,    // Start date/time
    pub end: Option<String>,      // End date/time
}

/// Meta information for resources
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiMeta {
    #[serde(rename = "versionId")]
    pub version_id: Option<String>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
    pub source: Option<String>,
    pub profile: Option<Vec<String>>,
    pub security: Option<Vec<EpiCoding>>,
    pub tag: Option<Vec<EpiCoding>>,
}

/// Extension for additional data
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EpiExtension {
    pub url: Option<String>,
    #[serde(rename = "valueCodeableConcept")]
    pub value_codeable_concept: Option<EpiCodeableConcept>,
    #[serde(rename = "valueString")]
    pub value_string: Option<String>,
    #[serde(rename = "valueBoolean")]
    pub value_boolean: Option<bool>,
    #[serde(rename = "valueDate")]
    pub value_date: Option<String>,
    pub extension: Option<Vec<Box<EpiExtension>>>,  // Box to break potential circular dependency
}

// ============================================================================
// Database Models
// ============================================================================

/// EMA catalog entry stored in database
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmaCatalogEntry {
    pub id: Uuid,
    pub eu_number: String,
    pub pms_id: Option<String>,
    pub bundle_id: Option<String>,
    pub epi_id: Option<String>,

    // Product identification
    pub product_name: String,
    pub inn_name: Option<String>,
    pub therapeutic_indication: Option<String>,

    // Marketing authorization
    pub mah_name: String,
    pub mah_country: Option<String>,
    pub authorization_status: Option<String>,
    pub authorization_date: Option<NaiveDate>,
    pub authorization_country: Option<String>,
    pub procedure_type: Option<String>,

    // Product characteristics
    pub pharmaceutical_form: Option<String>,
    pub route_of_administration: Option<Vec<String>>,
    pub strength: Option<String>,
    pub active_substances: Option<serde_json::Value>,
    pub excipients: Option<serde_json::Value>,

    // Classification
    pub atc_code: Option<String>,
    pub atc_classification: Option<String>,
    pub therapeutic_area: Option<String>,
    pub orphan_designation: Option<bool>,

    // Regulatory and safety
    pub pharmacovigilance_status: Option<String>,
    pub additional_monitoring: Option<bool>,
    pub risk_management_plan: Option<bool>,

    // Language and documentation
    pub language_code: Option<String>,
    pub epi_url: Option<String>,
    pub smpc_url: Option<String>,
    pub pil_url: Option<String>,

    // Raw data
    pub epi_data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,

    // Timestamps
    pub last_synced_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Simplified response model for client API
#[derive(Debug, Serialize, Clone)]
pub struct EmaCatalogResponse {
    pub id: Uuid,
    pub eu_number: String,
    pub product_name: String,
    pub inn_name: Option<String>,
    pub mah_name: String,
    pub pharmaceutical_form: Option<String>,
    pub strength: Option<String>,
    pub authorization_status: Option<String>,
    pub therapeutic_area: Option<String>,
    pub atc_code: Option<String>,
    pub orphan_designation: bool,
    pub language_code: String,
}

/// Search request parameters
#[derive(Debug, Deserialize)]
pub struct EmaSearchRequest {
    pub query: Option<String>,
    pub language: Option<String>,
    pub authorization_status: Option<String>,
    pub therapeutic_area: Option<String>,
    pub atc_code: Option<String>,
    pub mah_name: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Sync log entry for tracking synchronization operations
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EmaSyncLog {
    pub id: Uuid,
    pub sync_started_at: DateTime<Utc>,
    pub sync_completed_at: Option<DateTime<Utc>>,
    pub language_code: Option<String>,
    pub sync_type: Option<String>,
    pub record_limit: Option<i32>,
    pub records_fetched: Option<i32>,
    pub records_inserted: Option<i32>,
    pub records_updated: Option<i32>,
    pub records_skipped: Option<i32>,
    pub records_failed: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub warning_messages: Option<Vec<String>>,
    pub api_response_time_ms: Option<i32>,
    pub processing_time_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Statistics about the catalog
#[derive(Debug, Serialize, Clone)]
pub struct EmaCatalogStats {
    pub total_entries: i64,
    pub entries_by_language: Vec<LanguageCount>,
    pub entries_by_status: Vec<StatusCount>,
    pub entries_by_therapeutic_area: Vec<TherapeuticAreaCount>,
    pub orphan_medicines_count: i64,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_sync_status: Option<String>,
}

/// Count by language
#[derive(Debug, Serialize, Clone, FromRow)]
pub struct LanguageCount {
    pub language_code: String,
    pub count: i64,
}

/// Count by authorization status
#[derive(Debug, Serialize, Clone, FromRow)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

/// Count by therapeutic area
#[derive(Debug, Serialize, Clone, FromRow)]
pub struct TherapeuticAreaCount {
    pub therapeutic_area: String,
    pub count: i64,
}

// ============================================================================
// Conversion Implementations
// ============================================================================

impl EpiEntry {
    /// Convert EPI entry to database catalog entry
    pub fn to_catalog_entry(&self) -> Result<EmaCatalogEntry, Box<dyn std::error::Error>> {
        let resource = &self.resource;

        // Extract EU number or primary identifier
        let eu_number = self.extract_eu_number()
            .unwrap_or_else(|| format!("AUTO-{}", Uuid::new_v4()));

        // Extract product name (primary identifier)
        let product_name = self.extract_product_name()
            .unwrap_or_else(|| "Unknown Product".to_string());

        // Extract INN (generic name)
        let inn_name = self.extract_inn_name();

        // Extract MAH (Marketing Authorization Holder)
        let mah_name = self.extract_mah_name()
            .unwrap_or_else(|| "Unknown MAH".to_string());

        // Extract other fields
        let authorization_status = resource.status.clone();
        let language_code = resource.language.clone().unwrap_or_else(|| "en".to_string());

        Ok(EmaCatalogEntry {
            id: Uuid::new_v4(),
            eu_number,
            pms_id: self.extract_pms_id(),
            bundle_id: self.full_url.clone(),
            epi_id: resource.id.clone(),
            product_name,
            inn_name,
            therapeutic_indication: resource.description.clone(),
            mah_name,
            mah_country: None, // Would need deeper parsing
            authorization_status,
            authorization_date: self.extract_date(resource.approval_date.as_deref()),
            authorization_country: Some("EU".to_string()), // Default for EMA
            procedure_type: Some("Centralized".to_string()), // Default for EMA
            pharmaceutical_form: None, // Would need deeper parsing
            route_of_administration: None, // Would need deeper parsing
            strength: None, // Would need deeper parsing
            active_substances: None, // Would need deeper parsing from contained resources
            excipients: None, // Would need deeper parsing
            atc_code: self.extract_atc_code(),
            atc_classification: None, // Would need deeper parsing
            therapeutic_area: self.extract_therapeutic_area(),
            orphan_designation: Some(false), // Would need specific check
            pharmacovigilance_status: None, // Would need specific check
            additional_monitoring: Some(false), // Would need specific check
            risk_management_plan: Some(false), // Would need specific check
            language_code: Some(language_code),
            epi_url: self.full_url.clone(),
            smpc_url: None, // Would need specific link extraction
            pil_url: None, // Would need specific link extraction
            epi_data: serde_json::to_value(&self.resource).ok(),
            metadata: serde_json::to_value(&resource.meta).ok(),
            last_synced_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    /// Extract EU number from identifiers
    fn extract_eu_number(&self) -> Option<String> {
        let resource = &self.resource;

        if let Some(identifiers) = &resource.identifier {
            for identifier in identifiers {
                if let Some(system) = &identifier.system {
                    if system.contains("eu-number") || system.contains("ema.europa.eu") {
                        if let Some(value) = &identifier.value {
                            if value.starts_with("EU/") {
                                return Some(value.clone());
                            }
                        }
                    }
                }
            }
        }

        // Also check in extensions for EU number
        if let Some(extensions) = &resource.extension {
            for ext in extensions {
                if let Some(url) = &ext.url {
                    if url.contains("eu-number") {
                        if let Some(val) = &ext.value_string {
                            if val.starts_with("EU/") {
                                return Some(val.clone());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract PMS ID
    fn extract_pms_id(&self) -> Option<String> {
        let resource = &self.resource;

        if let Some(identifiers) = &resource.identifier {
            for identifier in identifiers {
                if let Some(system) = &identifier.system {
                    if system.contains("pms") || system.contains("product-management") {
                        return identifier.value.clone();
                    }
                }
            }
        }
        None
    }

    /// Extract product name
    fn extract_product_name(&self) -> Option<String> {
        let resource = &self.resource;

        // Try title first
        if let Some(title) = &resource.title {
            return match title {
                EpiCodeableConcept::Simple { text, .. } => text.clone(),
                EpiCodeableConcept::Text(s) => Some(s.clone()),
            };
        }

        // Then try name field
        if let Some(names) = &resource.name {
            if let Some(first_name) = names.first() {
                return match first_name {
                    EpiCodeableConcept::Simple { text, .. } => text.clone(),
                    EpiCodeableConcept::Text(s) => Some(s.clone()),
                };
            }
        }

        None
    }

    /// Extract INN (generic name)
    fn extract_inn_name(&self) -> Option<String> {
        let resource = &self.resource;

        // Check in contained resources for substances
        if let Some(contained) = &resource.contained {
            for item in contained {
                if let Some(obj) = item.as_object() {
                    if let Some(resource_type) = obj.get("resourceType").and_then(|v| v.as_str()) {
                        if resource_type == "MedicinalProductDefinition" {
                            if let Some(name) = obj.get("name").and_then(|v| v.as_array()) {
                                for name_item in name {
                                    if let Some(name_obj) = name_item.as_object() {
                                        if let Some(name_use) = name_obj.get("use").and_then(|v| v.as_object()) {
                                            if let Some(coding) = name_use.get("coding").and_then(|v| v.as_array()) {
                                                for code in coding {
                                                    if let Some(code_obj) = code.as_object() {
                                                        if let Some(code_str) = code_obj.get("code").and_then(|v| v.as_str()) {
                                                            if code_str == "INN" || code_str == "generic" {
                                                                return name_obj.get("productName").and_then(|v| v.as_str()).map(|s| s.to_string());
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract MAH name
    fn extract_mah_name(&self) -> Option<String> {
        let resource = &self.resource;

        // Check author field for MAH
        if let Some(authors) = &resource.author {
            for author in authors {
                if let Some(display) = &author.display {
                    return Some(display.clone());
                }
            }
        }

        // Check publisher field
        if let Some(publisher) = &resource.publisher {
            return Some(publisher.clone());
        }

        // Check contact information
        if let Some(contacts) = &resource.contact {
            for contact in contacts {
                if let Some(name) = &contact.name {
                    if let Some(text) = &name.text {
                        if !text.is_empty() {
                            return Some(text.clone());
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract ATC code
    fn extract_atc_code(&self) -> Option<String> {
        let resource = &self.resource;

        // Check code field for ATC classification
        if let Some(code) = &resource.code {
            return match code {
                EpiCodeableConcept::Simple { coding, .. } => {
                    if let Some(coding_list) = coding {
                        for coding_item in coding_list {
                            if let Some(system) = &coding_item.system {
                                if system.contains("who-ATC") || system.contains("atc") {
                                    return coding_item.code.clone();
                                }
                            }
                        }
                    }
                    None
                },
                _ => None,
            };
        }

        None
    }

    /// Extract therapeutic area
    fn extract_therapeutic_area(&self) -> Option<String> {
        let resource = &self.resource;

        // From domain field
        if let Some(domain) = &resource.domain {
            return match domain {
                EpiCodeableConcept::Simple { coding, .. } => {
                    if let Some(coding_list) = coding {
                        return coding_list.first()
                            .and_then(|c| c.display.clone())
                            .or_else(|| coding_list.first().and_then(|c| c.code.clone()));
                    }
                    None
                },
                EpiCodeableConcept::Text(s) => Some(s.clone()),
            };
        }

        // From therapeutic indication
        if let Some(indication) = &resource.description {
            return Some(indication.clone());
        }

        None
    }

    /// Helper to parse date strings
    fn extract_date(&self, date_str: Option<&str>) -> Option<NaiveDate> {
        date_str?.parse().ok()
    }
}

// ============================================================================
// Response Conversions
// ============================================================================

impl From<EmaCatalogEntry> for EmaCatalogResponse {
    fn from(entry: EmaCatalogEntry) -> Self {
        Self {
            id: entry.id,
            eu_number: entry.eu_number,
            product_name: entry.product_name,
            inn_name: entry.inn_name,
            mah_name: entry.mah_name,
            pharmaceutical_form: entry.pharmaceutical_form,
            strength: entry.strength,
            authorization_status: entry.authorization_status,
            therapeutic_area: entry.therapeutic_area,
            atc_code: entry.atc_code,
            orphan_designation: entry.orphan_designation.unwrap_or(false),
            language_code: entry.language_code.unwrap_or_else(|| "en".to_string()),
        }
    }
}
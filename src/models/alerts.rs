/// Alert and Notification System Models
///
/// Comprehensive models for smart inventory alerts including:
/// - User alert preferences
/// - Alert notifications
/// - Marketplace watchlist
/// - Alert processing logs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// ENUMS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    ExpiryWarning,
    ExpiryCritical,
    LowStock,
    WatchlistMatch,
    PriceDrop,
    NewInquiry,
    InquiryMessage,
    System,
}

impl AlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertType::ExpiryWarning => "expiry_warning",
            AlertType::ExpiryCritical => "expiry_critical",
            AlertType::LowStock => "low_stock",
            AlertType::WatchlistMatch => "watchlist_match",
            AlertType::PriceDrop => "price_drop",
            AlertType::NewInquiry => "new_inquiry",
            AlertType::InquiryMessage => "inquiry_message",
            AlertType::System => "system",
        }
    }
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl AlertSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AlertSeverity::Info => "info",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "critical",
        }
    }
}

impl std::fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
// DATABASE MODELS
// ============================================================================

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct UserAlertPreferences {
    pub user_id: Uuid,
    pub expiry_alerts_enabled: bool,
    pub expiry_alert_days: i32,
    pub low_stock_alerts_enabled: bool,
    pub low_stock_threshold: i32,
    pub watchlist_alerts_enabled: bool,
    pub email_notifications_enabled: bool,
    pub in_app_notifications_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AlertNotification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub inventory_id: Option<Uuid>,
    pub related_user_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub action_url: Option<String>,
    pub is_read: bool,
    pub is_dismissed: bool,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    pub dismissed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct MarketplaceWatchlist {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub search_criteria: serde_json::Value,
    pub alert_enabled: bool,
    pub last_checked_at: DateTime<Utc>,
    pub last_match_count: i32,
    pub total_matches_found: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct AlertProcessingLog {
    pub id: Uuid,
    pub run_type: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: String,
    pub alerts_generated: i32,
    pub errors_encountered: i32,
    pub error_details: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// API REQUEST MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct UpdateAlertPreferencesRequest {
    pub expiry_alerts_enabled: Option<bool>,
    pub expiry_alert_days: Option<i32>,
    pub low_stock_alerts_enabled: Option<bool>,
    pub low_stock_threshold: Option<i32>,
    pub watchlist_alerts_enabled: Option<bool>,
    pub email_notifications_enabled: Option<bool>,
    pub in_app_notifications_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateWatchlistRequest {
    pub name: String,
    pub description: Option<String>,
    pub search_criteria: serde_json::Value,
    pub alert_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWatchlistRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub search_criteria: Option<serde_json::Value>,
    pub alert_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct MarkAlertReadRequest {
    pub is_read: bool,
}

#[derive(Debug, Deserialize)]
pub struct GetNotificationsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub unread_only: Option<bool>,
    pub alert_type: Option<String>,
}

// ============================================================================
// API RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct AlertNotificationResponse {
    pub id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub inventory_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub action_url: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
    pub time_ago: String,
}

impl From<AlertNotification> for AlertNotificationResponse {
    fn from(notif: AlertNotification) -> Self {
        Self {
            id: notif.id,
            alert_type: notif.alert_type,
            severity: notif.severity,
            title: notif.title,
            message: notif.message,
            inventory_id: notif.inventory_id,
            metadata: notif.metadata,
            action_url: notif.action_url,
            is_read: notif.is_read,
            time_ago: format_time_ago(notif.created_at),
            created_at: notif.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct NotificationSummary {
    pub total_unread: i64,
    pub total_notifications: i64,
    pub notifications: Vec<AlertNotificationResponse>,
}

#[derive(Debug, Serialize)]
pub struct WatchlistResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub search_criteria: serde_json::Value,
    pub alert_enabled: bool,
    pub last_checked_at: DateTime<Utc>,
    pub last_match_count: i32,
    pub total_matches_found: i32,
    pub created_at: DateTime<Utc>,
}

impl From<MarketplaceWatchlist> for WatchlistResponse {
    fn from(watchlist: MarketplaceWatchlist) -> Self {
        Self {
            id: watchlist.id,
            name: watchlist.name,
            description: watchlist.description,
            search_criteria: watchlist.search_criteria,
            alert_enabled: watchlist.alert_enabled,
            last_checked_at: watchlist.last_checked_at,
            last_match_count: watchlist.last_match_count,
            total_matches_found: watchlist.total_matches_found,
            created_at: watchlist.created_at,
        }
    }
}

// ============================================================================
// INTERNAL MODELS (for alert generation)
// ============================================================================

#[derive(Debug, Clone)]
pub struct AlertPayload {
    pub user_id: Uuid,
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub inventory_id: Option<Uuid>,
    pub related_user_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub action_url: Option<String>,
}

impl AlertPayload {
    pub fn new_expiry_warning(
        user_id: Uuid,
        inventory_id: Uuid,
        product_name: &str,
        days_to_expiry: i64,
        quantity: i32,
    ) -> Self {
        let severity = if days_to_expiry <= 7 {
            AlertSeverity::Critical
        } else {
            AlertSeverity::Warning
        };

        let alert_type = if days_to_expiry <= 7 {
            AlertType::ExpiryCritical
        } else {
            AlertType::ExpiryWarning
        };

        Self {
            user_id,
            alert_type,
            severity,
            title: format!("Expiry Alert: {} expires in {} days", product_name, days_to_expiry),
            message: format!(
                "Your inventory of {} ({} units) will expire in {} days. Consider pricing adjustments or promotional sales.",
                product_name, quantity, days_to_expiry
            ),
            inventory_id: Some(inventory_id),
            related_user_id: None,
            metadata: Some(serde_json::json!({
                "days_to_expiry": days_to_expiry,
                "quantity": quantity,
                "product_name": product_name,
            })),
            action_url: Some(format!("/dashboard/inventory?highlight={}", inventory_id)),
        }
    }

    pub fn new_low_stock(
        user_id: Uuid,
        inventory_id: Uuid,
        product_name: &str,
        current_quantity: i32,
        threshold: i32,
    ) -> Self {
        Self {
            user_id,
            alert_type: AlertType::LowStock,
            severity: AlertSeverity::Warning,
            title: format!("Low Stock: {}", product_name),
            message: format!(
                "Your inventory of {} is running low ({} units remaining, below threshold of {}).",
                product_name, current_quantity, threshold
            ),
            inventory_id: Some(inventory_id),
            related_user_id: None,
            metadata: Some(serde_json::json!({
                "current_quantity": current_quantity,
                "threshold": threshold,
                "product_name": product_name,
            })),
            action_url: Some(format!("/dashboard/inventory?highlight={}", inventory_id)),
        }
    }

    pub fn new_watchlist_match(
        user_id: Uuid,
        watchlist_name: &str,
        match_count: i32,
        inventory_id: Option<Uuid>,
    ) -> Self {
        Self {
            user_id,
            alert_type: AlertType::WatchlistMatch,
            severity: AlertSeverity::Info,
            title: format!("New matches for watchlist: {}", watchlist_name),
            message: format!(
                "We found {} new marketplace listing(s) matching your saved search \"{}\".",
                match_count, watchlist_name
            ),
            inventory_id,
            related_user_id: None,
            metadata: Some(serde_json::json!({
                "watchlist_name": watchlist_name,
                "match_count": match_count,
            })),
            action_url: Some("/dashboard/marketplace".to_string()),
        }
    }

    /// Create a new inquiry notification for the seller
    pub fn new_inquiry(
        seller_id: Uuid,
        buyer_id: Uuid,
        buyer_company: &str,
        product_name: &str,
        quantity: i32,
        inquiry_id: Uuid,
        inventory_id: Uuid,
    ) -> Self {
        Self {
            user_id: seller_id,
            alert_type: AlertType::NewInquiry,
            severity: AlertSeverity::Info,
            title: format!("New inquiry from {}", buyer_company),
            message: format!(
                "{} has inquired about {} units of {}.",
                buyer_company, quantity, product_name
            ),
            inventory_id: Some(inventory_id),
            related_user_id: Some(buyer_id),
            metadata: Some(serde_json::json!({
                "inquiry_id": inquiry_id,
                "buyer_company": buyer_company,
                "product_name": product_name,
                "quantity": quantity,
            })),
            action_url: Some(format!("/dashboard/inquiries?id={}", inquiry_id)),
        }
    }

    /// Create a new message notification
    pub fn new_inquiry_message(
        recipient_id: Uuid,
        sender_id: Uuid,
        sender_company: &str,
        inquiry_id: Uuid,
    ) -> Self {
        Self {
            user_id: recipient_id,
            alert_type: AlertType::InquiryMessage,
            severity: AlertSeverity::Info,
            title: format!("New message from {}", sender_company),
            message: format!("{} sent you a message regarding an inquiry.", sender_company),
            inventory_id: None,
            related_user_id: Some(sender_id),
            metadata: Some(serde_json::json!({
                "inquiry_id": inquiry_id,
                "sender_company": sender_company,
            })),
            action_url: Some(format!("/dashboard/inquiries?id={}", inquiry_id)),
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Format timestamp as "X minutes ago", "X hours ago", etc.
fn format_time_ago(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    if duration.num_seconds() < 60 {
        "Just now".to_string()
    } else if duration.num_minutes() < 60 {
        let mins = duration.num_minutes();
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if duration.num_hours() < 24 {
        let hours = duration.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if duration.num_days() < 7 {
        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if duration.num_weeks() < 4 {
        let weeks = duration.num_weeks();
        format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
    } else {
        timestamp.format("%b %d, %Y").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_type_as_str() {
        assert_eq!(AlertType::ExpiryWarning.as_str(), "expiry_warning");
        assert_eq!(AlertType::LowStock.as_str(), "low_stock");
    }

    #[test]
    fn test_alert_severity_as_str() {
        assert_eq!(AlertSeverity::Info.as_str(), "info");
        assert_eq!(AlertSeverity::Warning.as_str(), "warning");
        assert_eq!(AlertSeverity::Critical.as_str(), "critical");
    }

    #[test]
    fn test_expiry_alert_payload_creation() {
        let user_id = Uuid::new_v4();
        let inventory_id = Uuid::new_v4();
        let payload = AlertPayload::new_expiry_warning(
            user_id,
            inventory_id,
            "Amoxicillin 500mg",
            5,
            100,
        );

        assert_eq!(payload.user_id, user_id);
        assert_eq!(payload.alert_type, AlertType::ExpiryCritical);
        assert_eq!(payload.severity, AlertSeverity::Critical);
        assert!(payload.title.contains("expires in 5 days"));
    }
}

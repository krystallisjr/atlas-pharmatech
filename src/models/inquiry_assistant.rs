/// Inquiry Assistant models for AI-powered inquiry response suggestions

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============================================================================
// Database Models
// ============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct InquiryAiSuggestion {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub user_id: Uuid,
    pub suggestion_type: String,
    pub suggestion_text: String,
    pub context_snapshot: Option<serde_json::Value>,
    pub ai_reasoning: Option<String>,
    pub ai_cost_usd: rust_decimal::Decimal,
    pub ai_tokens_used: i32,
    pub was_accepted: bool,
    pub was_edited: bool,
    pub final_message_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// API Request/Response Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GenerateSuggestionRequest {
    pub suggestion_type: SuggestionType,
    pub custom_instructions: Option<String>, // User can guide AI: "be more formal", "offer 10% discount", etc.
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    InitialResponse,
    Negotiation,
    PricingAdjustment,
    TermsClarification,
    ClosingDeal,
    FollowUp,
    Rejection,
}

impl ToString for SuggestionType {
    fn to_string(&self) -> String {
        match self {
            SuggestionType::InitialResponse => "initial_response".to_string(),
            SuggestionType::Negotiation => "negotiation".to_string(),
            SuggestionType::PricingAdjustment => "pricing_adjustment".to_string(),
            SuggestionType::TermsClarification => "terms_clarification".to_string(),
            SuggestionType::ClosingDeal => "closing_deal".to_string(),
            SuggestionType::FollowUp => "follow_up".to_string(),
            SuggestionType::Rejection => "rejection".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SuggestionResponse {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub suggestion_type: String,
    pub suggestion_text: String,
    pub reasoning: Option<String>,
    pub context_used: InquiryContext,
    pub ai_cost_usd: String,
    pub created_at: DateTime<Utc>,
}

impl From<InquiryAiSuggestion> for SuggestionResponse {
    fn from(sug: InquiryAiSuggestion) -> Self {
        let context = sug.context_snapshot
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        Self {
            id: sug.id,
            inquiry_id: sug.inquiry_id,
            suggestion_type: sug.suggestion_type,
            suggestion_text: sug.suggestion_text,
            reasoning: sug.ai_reasoning,
            context_used: context,
            ai_cost_usd: sug.ai_cost_usd.to_string(),
            created_at: sug.created_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct InquiryContext {
    pub product_name: Option<String>,
    pub quantity_requested: i32,
    pub quantity_available: i32,
    pub unit_price: Option<f64>,
    pub batch_number: Option<String>,
    pub expiry_date: Option<String>,
    pub buyer_company: String,
    pub seller_company: String,
    pub message_count: i32,
    pub inquiry_status: String,
}

#[derive(Debug, Deserialize)]
pub struct AcceptSuggestionRequest {
    pub edited_text: Option<String>, // If user edited the suggestion before sending
}

#[derive(Debug, Serialize)]
pub struct AcceptSuggestionResponse {
    pub message_id: Uuid,
    pub was_edited: bool,
}

#[derive(Debug, Serialize)]
pub struct SuggestionHistoryResponse {
    pub suggestions: Vec<SuggestionHistoryItem>,
    pub total_cost_usd: String,
    pub suggestions_remaining: i32,
}

#[derive(Debug, Serialize)]
pub struct SuggestionHistoryItem {
    pub id: Uuid,
    pub inquiry_id: Uuid,
    pub suggestion_type: String,
    pub was_accepted: bool,
    pub was_edited: bool,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Internal Models for AI Processing
// ============================================================================

#[derive(Debug, Serialize)]
pub struct ConversationHistory {
    pub messages: Vec<ConversationMessage>,
}

#[derive(Debug, Serialize)]
pub struct ConversationMessage {
    pub sender: String, // "buyer" or "seller"
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

/// Natural Language Query models for AI-powered database querying

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// ============================================================================
// Database Models
// ============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct NlQuerySession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub query_text: String,
    pub generated_sql: Option<String>,
    pub validated_sql: Option<String>,
    pub execution_time_ms: Option<i32>,
    pub result_count: Option<i32>,
    pub result_data: Option<serde_json::Value>,
    pub ai_cost_usd: rust_decimal::Decimal,
    pub ai_tokens_used: i32,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct NlQueryFavorite {
    pub id: Uuid,
    pub user_id: Uuid,
    pub query_text: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// API Request/Response Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ExecuteQueryRequest {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub id: Uuid,
    pub status: String,
    pub query_text: String,
    pub response_type: String, // "sql_query" or "conversation"
    pub generated_sql: Option<String>,
    pub execution_time_ms: Option<i32>,
    pub result_count: Option<i32>,
    pub results: Option<Vec<serde_json::Value>>,
    pub explanation: Option<String>,
    pub ai_response: Option<String>, // For conversational responses
    pub ai_cost_usd: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<NlQuerySession> for QueryResponse {
    fn from(session: NlQuerySession) -> Self {
        // Detect if this is a conversational response or SQL query
        let (response_type, results, ai_response) = if let Some(ref data) = session.result_data {
            if let Some(answer) = data.get("answer").and_then(|v| v.as_str()) {
                // Conversational response
                ("conversation".to_string(), None, Some(answer.to_string()))
            } else if let serde_json::Value::Array(arr) = data {
                // SQL query results
                ("sql_query".to_string(), Some(arr.clone()), None)
            } else {
                // Fallback
                ("sql_query".to_string(), None, None)
            }
        } else {
            ("sql_query".to_string(), None, None)
        };

        Self {
            id: session.id,
            status: session.status,
            query_text: session.query_text,
            response_type,
            generated_sql: session.validated_sql.or(session.generated_sql),
            execution_time_ms: session.execution_time_ms,
            result_count: session.result_count,
            results,
            explanation: None,
            ai_response,
            ai_cost_usd: session.ai_cost_usd.to_string(),
            error_message: session.error_message,
            created_at: session.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct QueryHistoryResponse {
    pub sessions: Vec<QueryHistoryItem>,
    pub total_cost_usd: String,
    pub queries_remaining: i32,
}

#[derive(Debug, Serialize)]
pub struct QueryHistoryItem {
    pub id: Uuid,
    pub query_text: String,
    pub status: String,
    pub result_count: Option<i32>,
    pub execution_time_ms: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SaveFavoriteRequest {
    pub query_text: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FavoriteResponse {
    pub id: Uuid,
    pub query_text: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<NlQueryFavorite> for FavoriteResponse {
    fn from(fav: NlQueryFavorite) -> Self {
        Self {
            id: fav.id,
            query_text: fav.query_text,
            description: fav.description,
            category: fav.category,
            created_at: fav.created_at,
        }
    }
}

// ============================================================================
// AI Response Models
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AiSqlResponse {
    pub sql: String,
    pub explanation: Option<String>,
    pub parameters: Option<Vec<serde_json::Value>>,
}

/// Hybrid AI response that can be either conversational or data-driven
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiAssistantResponse {
    /// Conversational response - no database query needed
    Conversation {
        answer: String,
        #[serde(default)]
        follow_up_suggestions: Vec<String>,
    },
    /// SQL query response - needs database execution
    SqlQuery {
        sql: String,
        explanation: Option<String>,
        #[serde(default)]
        parameters: Option<Vec<serde_json::Value>>,
    },
}

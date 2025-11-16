/// Production-grade Claude AI service for Atlas Pharma
/// Handles all AI-powered features with cost tracking, rate limiting, and audit trails

use serde::{Deserialize, Serialize};
use crate::middleware::error_handling::{Result, AppError};
use std::time::Instant;
use sqlx::PgPool;
use uuid::Uuid;

// Default to official Anthropic API, but can be overridden with env var for proxies like z.ai
const DEFAULT_CLAUDE_API_URL: &str = "https://api.anthropic.com/v1/messages";
const CLAUDE_MODEL: &str = "claude-3-5-sonnet-20241022";
const CLAUDE_VERSION: &str = "2023-06-01";

// Pricing per million tokens (as of 2025)
const INPUT_COST_PER_MILLION: f64 = 3.0;
const OUTPUT_COST_PER_MILLION: f64 = 15.0;

// ============================================================================
// Request/Response Models
// ============================================================================

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    system: Option<String>,
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClaudeMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: String,
}

#[derive(Debug, Deserialize, Clone)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

// ============================================================================
// Public API Models
// ============================================================================

#[derive(Debug)]
pub struct ClaudeApiResponse {
    pub content: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
    pub latency_ms: u64,
}

/// Configuration for Claude AI requests
pub struct ClaudeRequestConfig {
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
}

impl Default for ClaudeRequestConfig {
    fn default() -> Self {
        Self {
            max_tokens: 4096,
            temperature: Some(1.0),
            system_prompt: None,
        }
    }
}

// ============================================================================
// Claude AI Service
// ============================================================================

pub struct ClaudeAIService {
    api_key: String,
    http_client: reqwest::Client,
    db_pool: PgPool,
}

impl ClaudeAIService {
    pub fn new(api_key: String, db_pool: PgPool) -> Self {
        Self {
            api_key,
            http_client: reqwest::Client::new(),
            db_pool,
        }
    }

    /// Main method to send a request to Claude
    pub async fn send_message(
        &self,
        messages: Vec<ClaudeMessage>,
        config: ClaudeRequestConfig,
        user_id: Uuid,
        session_id: Option<Uuid>,
    ) -> Result<ClaudeApiResponse> {
        // CRITICAL: Check quota BEFORE making API call (prevents cost attacks)
        if !self.check_and_reserve_quota(user_id).await? {
            return Err(AppError::QuotaExceeded(
                "Monthly AI usage limit exceeded. Please upgrade your plan or wait for monthly reset.".to_string()
            ));
        }

        let start_time = Instant::now();

        // Build request
        let request = ClaudeRequest {
            model: CLAUDE_MODEL.to_string(),
            max_tokens: config.max_tokens,
            messages,
            system: config.system_prompt,
            temperature: config.temperature,
        };

        // Get API URL from env or use default
        let api_url = std::env::var("ANTHROPIC_BASE_URL")
            .unwrap_or_else(|_| DEFAULT_CLAUDE_API_URL.to_string());

        // Send to Claude API (or proxy like z.ai)
        let response = self.http_client
            .post(&api_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", CLAUDE_VERSION)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Claude API request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            tracing::error!("Claude API error ({}): {}", status, error_body);
            return Err(AppError::Internal(anyhow::anyhow!(
                "Claude API returned error {}: {}",
                status,
                error_body
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to parse Claude response: {}", e)))?;

        let latency_ms = start_time.elapsed().as_millis() as u64;

        // Extract content
        let content = claude_response.content
            .into_iter()
            .find(|block| block.block_type == "text")
            .map(|block| block.text)
            .unwrap_or_default();

        // Calculate costs
        let input_cost = (claude_response.usage.input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
        let output_cost = (claude_response.usage.output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;
        let total_cost = input_cost + output_cost;

        // Log usage to database
        self.log_api_usage(
            user_id,
            session_id,
            claude_response.usage.clone(),
            total_cost,
            latency_ms,
            status.as_u16() as i32,
        ).await?;

        tracing::info!(
            "Claude API call: user={}, tokens_in={}, tokens_out={}, cost=${:.6}, latency={}ms",
            user_id,
            claude_response.usage.input_tokens,
            claude_response.usage.output_tokens,
            total_cost,
            latency_ms
        );

        Ok(ClaudeApiResponse {
            content,
            input_tokens: claude_response.usage.input_tokens,
            output_tokens: claude_response.usage.output_tokens,
            cost_usd: total_cost,
            latency_ms,
        })
    }

    /// Check quota AND reserve slot atomically (prevents race conditions)
    /// Returns true if quota available and reserved, false if exceeded
    async fn check_and_reserve_quota(&self, user_id: Uuid) -> Result<bool> {
        // Use transaction with SELECT FOR UPDATE to prevent concurrent quota bypass
        let mut tx = self.db_pool.begin().await?;

        // Ensure user limits exist
        sqlx::query!(
            r#"
            INSERT INTO user_ai_usage_limits (user_id)
            VALUES ($1)
            ON CONFLICT (user_id) DO NOTHING
            "#,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        // Get user limits with row lock
        let limits = sqlx::query!(
            r#"
            SELECT
                monthly_import_limit,
                monthly_imports_used,
                monthly_ai_cost_limit_usd,
                monthly_ai_cost_used_usd,
                limit_period_end
            FROM user_ai_usage_limits
            WHERE user_id = $1
            FOR UPDATE
            "#,
            user_id
        )
        .fetch_one(&mut *tx)
        .await?;

        // Check if limits exceeded
        let has_import_quota = limits.monthly_imports_used < limits.monthly_import_limit;
        let has_cost_quota = limits.monthly_ai_cost_used_usd < limits.monthly_ai_cost_limit_usd;

        if !has_import_quota || !has_cost_quota {
            // Quota exceeded - rollback and return false
            tx.rollback().await?;
            return Ok(false);
        }

        // Reserve slot by incrementing counter
        sqlx::query!(
            r#"
            UPDATE user_ai_usage_limits
            SET monthly_imports_used = monthly_imports_used + 1,
                updated_at = NOW()
            WHERE user_id = $1
            "#,
            user_id
        )
        .execute(&mut *tx)
        .await?;

        // Commit reservation
        tx.commit().await?;

        Ok(true)
    }

    /// Check if user has available quota (read-only, no reservation)
    pub async fn check_user_quota(&self, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query!(
            r#"
            SELECT
                monthly_imports_used < monthly_import_limit as has_monthly_quota,
                monthly_ai_cost_used_usd < monthly_ai_cost_limit_usd as has_cost_quota
            FROM user_ai_usage_limits
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        // If no limits set, allow (defaults will be applied on first use)
        Ok(result.map(|r| r.has_monthly_quota.unwrap_or(true) && r.has_cost_quota.unwrap_or(true)).unwrap_or(true))
    }

    /// Update cost after API call completes (import already reserved in check_and_reserve_quota)
    pub async fn increment_user_usage(&self, user_id: Uuid, cost_usd: f64) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE user_ai_usage_limits
            SET monthly_ai_cost_used_usd = monthly_ai_cost_used_usd + $2,
                updated_at = NOW()
            WHERE user_id = $1
            "#,
            user_id,
            rust_decimal::Decimal::try_from(cost_usd).unwrap_or_default()
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    /// Log API usage for cost tracking and analytics
    async fn log_api_usage(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
        usage: Usage,
        cost_usd: f64,
        latency_ms: u64,
        status_code: i32,
    ) -> Result<()> {
        let input_cost = (usage.input_tokens as f64 / 1_000_000.0) * INPUT_COST_PER_MILLION;
        let output_cost = (usage.output_tokens as f64 / 1_000_000.0) * OUTPUT_COST_PER_MILLION;

        sqlx::query!(
            r#"
            INSERT INTO ai_api_usage (
                user_id,
                session_id,
                api_provider,
                api_model,
                api_endpoint,
                input_tokens,
                output_tokens,
                total_tokens,
                input_cost_usd,
                output_cost_usd,
                total_cost_usd,
                latency_ms,
                status_code
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            "#,
            user_id,
            session_id,
            "anthropic",
            CLAUDE_MODEL,
            "/v1/messages",
            usage.input_tokens as i32,
            usage.output_tokens as i32,
            (usage.input_tokens + usage.output_tokens) as i32,
            rust_decimal::Decimal::try_from(input_cost).unwrap_or_default(),
            rust_decimal::Decimal::try_from(output_cost).unwrap_or_default(),
            rust_decimal::Decimal::try_from(cost_usd).unwrap_or_default(),
            latency_ms as i32,
            status_code
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a user message for Claude
pub fn user_message(content: impl Into<String>) -> ClaudeMessage {
    ClaudeMessage {
        role: "user".to_string(),
        content: content.into(),
    }
}

/// Create an assistant message for Claude (for conversation history)
pub fn assistant_message(content: impl Into<String>) -> ClaudeMessage {
    ClaudeMessage {
        role: "assistant".to_string(),
        content: content.into(),
    }
}

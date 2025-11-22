// ============================================================================
// API Quota Service - Usage Monitoring and Rate Limiting
// ============================================================================
//
// 🔒 SECURITY: Prevents API key abuse and tracks usage for cost control
//
// ## Problem:
// Exposed API keys (Anthropic, OpenAI, etc.) can lead to:
// - Unlimited API usage if key leaked
// - Financial cost to company
// - Potential data exfiltration via AI prompts
// - Rate limit exhaustion
//
// ## Solution: Usage Quotas and Monitoring
//
// **Features:**
// 1. Per-user quota limits
// 2. Per-endpoint tracking
// 3. Cost estimation
// 4. Usage analytics
// 5. Anomaly detection
// 6. Alert thresholds
//
// ## Quota Tiers:
//
// - Free: 100 AI requests/month
// - Basic: 1,000 requests/month
// - Pro: 10,000 requests/month
// - Enterprise: Unlimited
//
// ## Monitoring:
//
// - Track tokens used (input + output)
// - Estimate costs per request
// - Alert on unusual patterns
// - Daily/monthly usage reports
//
// ============================================================================

use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};
use crate::middleware::error_handling::{Result, AppError};

/// User quota tier - matches PostgreSQL enum quota_tier exactly
#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "quota_tier")]
pub enum QuotaTier {
    Free,       // 100 requests/month
    Basic,      // 1,000 requests/month
    Pro,        // 10,000 requests/month
    Enterprise, // Unlimited
}

impl QuotaTier {
    /// Get monthly request limit for tier
    pub fn monthly_limit(&self) -> Option<i32> {
        match self {
            QuotaTier::Free => Some(100),
            QuotaTier::Basic => Some(1_000),
            QuotaTier::Pro => Some(10_000),
            QuotaTier::Enterprise => None, // Unlimited
        }
    }

    /// Get cost per 1K tokens (in USD cents)
    pub fn token_cost_cents(&self) -> f64 {
        match self {
            QuotaTier::Free => 0.0,      // Free tier (we absorb cost)
            QuotaTier::Basic => 0.5,     // $0.005 per 1K tokens
            QuotaTier::Pro => 0.3,       // $0.003 per 1K tokens (discounted)
            QuotaTier::Enterprise => 0.2, // $0.002 per 1K tokens (max discount)
        }
    }
}

/// API usage record
#[derive(Debug, Clone)]
pub struct ApiUsageRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub endpoint: String,
    pub tokens_input: i32,
    pub tokens_output: i32,
    pub cost_cents: f64,
    pub latency_ms: i32,
    pub created_at: DateTime<Utc>,
}

/// Monthly usage summary
#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    pub user_id: Uuid,
    pub year: i32,
    pub month: i32,
    pub quota_tier: QuotaTier,
    pub total_requests: i32,
    pub total_tokens_input: i64,
    pub total_tokens_output: i64,
    pub total_cost_cents: f64,
    pub quota_limit: Option<i32>,
    pub quota_remaining: Option<i32>,
    pub quota_usage_percent: f64,
}

pub struct ApiQuotaService {
    db_pool: PgPool,
}

impl ApiQuotaService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Initialize API quota tracking system
    ///
    /// Creates necessary database tables
    ///
    pub async fn initialize(&self) -> Result<()> {
        sqlx::query(
            r#"
            -- User quota configuration
            CREATE TABLE IF NOT EXISTS user_api_quotas (
                user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                quota_tier TEXT NOT NULL DEFAULT 'free',
                custom_monthly_limit INTEGER,  -- Override for custom limits
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

                CONSTRAINT check_quota_tier CHECK (quota_tier IN ('free', 'basic', 'pro', 'enterprise'))
            );

            -- API usage tracking
            CREATE TABLE IF NOT EXISTS api_usage_log (
                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                endpoint TEXT NOT NULL,  -- e.g., "ai_import", "nl_query", "regulatory_gen"
                tokens_input INTEGER NOT NULL DEFAULT 0,
                tokens_output INTEGER NOT NULL DEFAULT 0,
                cost_cents DECIMAL(10,4) NOT NULL DEFAULT 0,
                latency_ms INTEGER NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_api_usage_user_date
                ON api_usage_log(user_id, created_at DESC);

            CREATE INDEX IF NOT EXISTS idx_api_usage_endpoint
                ON api_usage_log(endpoint, created_at DESC);

            -- Monthly usage summary (materialized view for performance)
            CREATE MATERIALIZED VIEW IF NOT EXISTS api_usage_monthly AS
            SELECT
                user_id,
                EXTRACT(YEAR FROM created_at)::INTEGER as year,
                EXTRACT(MONTH FROM created_at)::INTEGER as month,
                COUNT(*) as total_requests,
                SUM(tokens_input) as total_tokens_input,
                SUM(tokens_output) as total_tokens_output,
                SUM(cost_cents) as total_cost_cents,
                AVG(latency_ms) as avg_latency_ms
            FROM api_usage_log
            GROUP BY user_id, EXTRACT(YEAR FROM created_at), EXTRACT(MONTH FROM created_at);

            CREATE UNIQUE INDEX IF NOT EXISTS idx_api_usage_monthly_unique
                ON api_usage_monthly(user_id, year, month);

            COMMENT ON TABLE user_api_quotas IS
                'User quota tiers and limits for AI API usage';
            COMMENT ON TABLE api_usage_log IS
                'Detailed log of all AI API requests with cost tracking';
            "#
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!("✅ API quota tracking system initialized");

        Ok(())
    }

    /// Get or create user quota configuration
    ///
    /// Defaults to Free tier if user doesn't have quota configured
    ///
    pub async fn get_user_quota(&self, user_id: Uuid) -> Result<QuotaTier> {
        let quota = sqlx::query_scalar!(
            r#"
            SELECT quota_tier as "quota_tier: QuotaTier"
            FROM user_api_quotas
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match quota {
            Some(tier) => Ok(tier),
            None => {
                // Create default quota (Free tier)
                sqlx::query!(
                    r#"
                    INSERT INTO user_api_quotas (user_id, quota_tier)
                    VALUES ($1, 'Free')
                    ON CONFLICT (user_id) DO NOTHING
                    "#,
                    user_id
                )
                .execute(&self.db_pool)
                .await?;

                Ok(QuotaTier::Free)
            }
        }
    }

    /// Check if user has quota remaining for this month
    ///
    /// Returns (allowed, requests_used, requests_remaining)
    ///
    pub async fn check_quota(&self, user_id: Uuid) -> Result<(bool, i32, Option<i32>)> {
        let tier = self.get_user_quota(user_id).await?;
        let limit = tier.monthly_limit();

        // Get current month usage
        let now = Utc::now();
        let requests_used = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)::INTEGER as count
            FROM api_usage_log
            WHERE user_id = $1
              AND EXTRACT(YEAR FROM created_at) = $2
              AND EXTRACT(MONTH FROM created_at) = $3
            "#,
            user_id,
            now.year() as f64,
            now.month() as f64
        )
        .fetch_one(&self.db_pool)
        .await?
        .unwrap_or(0);

        match limit {
            Some(max_requests) => {
                let remaining = max_requests - requests_used;
                let allowed = remaining > 0;

                if !allowed {
                    tracing::warn!(
                        "⚠️  API QUOTA EXCEEDED - User: {}, Tier: {:?}, Used: {}/{}",
                        user_id,
                        tier,
                        requests_used,
                        max_requests
                    );
                }

                Ok((allowed, requests_used, Some(remaining)))
            }
            None => {
                // Unlimited (Enterprise tier)
                Ok((true, requests_used, None))
            }
        }
    }

    /// Record API usage
    ///
    /// Logs request details and updates usage tracking
    ///
    pub async fn record_usage(
        &self,
        user_id: Uuid,
        endpoint: &str,
        tokens_input: i32,
        tokens_output: i32,
        latency_ms: i32,
    ) -> Result<()> {
        // Get user tier for cost calculation
        let tier = self.get_user_quota(user_id).await?;
        let total_tokens = tokens_input + tokens_output;
        let cost_cents_f64 = (total_tokens as f64 / 1000.0) * tier.token_cost_cents();
        let cost_cents = rust_decimal::Decimal::from_f64_retain(cost_cents_f64).unwrap_or_default();

        // Insert usage record
        sqlx::query!(
            r#"
            INSERT INTO api_usage_log
            (user_id, endpoint, tokens_input, tokens_output, cost_cents, latency_ms)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            user_id,
            endpoint,
            tokens_input,
            tokens_output,
            cost_cents,
            latency_ms as i64
        )
        .execute(&self.db_pool)
        .await?;

        let cost_dollars = cost_cents_f64 / 100.0;

        tracing::info!(
            "📊 API Usage: user={}, endpoint={}, tokens={} (in:{} out:{}), cost=${:.4}, latency={}ms",
            user_id,
            endpoint,
            total_tokens,
            tokens_input,
            tokens_output,
            cost_dollars,
            latency_ms
        );

        Ok(())
    }

    /// Get monthly usage summary for user
    ///
    /// Returns detailed usage statistics
    ///
    pub async fn get_monthly_summary(&self, user_id: Uuid) -> Result<UsageSummary> {
        let now = Utc::now();
        let tier = self.get_user_quota(user_id).await?;
        let limit = tier.monthly_limit();

        let summary = sqlx::query!(
            r#"
            SELECT
                COUNT(*)::INTEGER as total_requests,
                COALESCE(SUM(tokens_input), 0) as total_tokens_input,
                COALESCE(SUM(tokens_output), 0) as total_tokens_output,
                COALESCE(SUM(cost_cents), 0) as total_cost_cents
            FROM api_usage_log
            WHERE user_id = $1
              AND EXTRACT(YEAR FROM created_at) = $2
              AND EXTRACT(MONTH FROM created_at) = $3
            "#,
            user_id,
            now.year() as f64,
            now.month() as f64
        )
        .fetch_one(&self.db_pool)
        .await?;

        let total_requests = summary.total_requests.unwrap_or(0);
        let quota_remaining = limit.map(|l| l - total_requests);
        let quota_usage_percent = match limit {
            Some(l) => (total_requests as f64 / l as f64 * 100.0).min(100.0),
            None => 0.0,
        };

        Ok(UsageSummary {
            user_id,
            year: now.year(),
            month: now.month() as i32,
            quota_tier: tier,
            total_requests,
            total_tokens_input: summary.total_tokens_input.unwrap_or(0),
            total_tokens_output: summary.total_tokens_output.unwrap_or(0),
            total_cost_cents: summary.total_cost_cents.unwrap_or(rust_decimal::Decimal::ZERO).to_string().parse().unwrap_or(0.0),
            quota_limit: limit,
            quota_remaining,
            quota_usage_percent,
        })
    }

    /// Detect usage anomalies
    ///
    /// Returns true if usage pattern is suspicious
    ///
    pub async fn detect_anomaly(&self, user_id: Uuid) -> Result<bool> {
        // Get last 24 hours usage
        let recent_usage = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*)::INTEGER
            FROM api_usage_log
            WHERE user_id = $1
              AND created_at > NOW() - INTERVAL '24 hours'
            "#,
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?
        .unwrap_or(0);

        // 🔒 SECURITY: Alert if unusual spike (>100 requests in 24h)
        const ANOMALY_THRESHOLD: i32 = 100;

        if recent_usage > ANOMALY_THRESHOLD {
            tracing::warn!(
                "⚠️  USAGE ANOMALY DETECTED - User: {}, Requests (24h): {} (threshold: {})",
                user_id,
                recent_usage,
                ANOMALY_THRESHOLD
            );
            return Ok(true);
        }

        Ok(false)
    }

    /// Upgrade user quota tier
    ///
    /// For admin use or subscription changes
    ///
    pub async fn upgrade_tier(&self, user_id: Uuid, new_tier: QuotaTier) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO user_api_quotas (user_id, quota_tier, updated_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (user_id)
            DO UPDATE SET quota_tier = $2, updated_at = NOW()
            "#,
            user_id,
            new_tier as QuotaTier
        )
        .execute(&self.db_pool)
        .await?;

        tracing::info!(
            "✅ User {} quota upgraded to {:?}",
            user_id,
            new_tier
        );

        Ok(())
    }

    /// Initialize default quotas for all existing users
    ///
    /// Should be called once at application startup
    /// Sets Free tier for users without quotas
    ///
    pub async fn initialize_default_quotas(&self) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO user_api_quotas (user_id, quota_tier, created_at, updated_at)
            SELECT id, 'Free', NOW(), NOW()
            FROM users
            WHERE id NOT IN (SELECT user_id FROM user_api_quotas)
            "#
        )
        .execute(&self.db_pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}

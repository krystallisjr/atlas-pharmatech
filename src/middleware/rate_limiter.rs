/// Production-grade rate limiting middleware for AI import endpoints and general API protection
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{Response, IntoResponse},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::{error_handling::AppError, Claims};
// Note: tower_governor has been replaced with custom ip_rate_limiter.rs
// These functions are kept for backward compatibility but are deprecated

/// Rate limit middleware - enforces hourly upload limits
pub async fn rate_limit_middleware(
    State(pool): State<PgPool>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract user from JWT claims (set by auth_middleware)
    let claims = request.extensions().get::<Claims>().cloned();

    if let Some(claims) = claims {
        // Check hourly rate limit
        if !check_hourly_limit(&pool, claims.user_id).await? {
            return Err(AppError::TooManyRequests(
                "Hourly upload limit exceeded. Please wait before uploading more files.".to_string()
            ));
        }

        // Increment hourly counter
        increment_hourly_usage(&pool, claims.user_id).await?;
    }

    Ok(next.run(request).await)
}

/// Check if user has available hourly quota
async fn check_hourly_limit(pool: &PgPool, user_id: Uuid) -> Result<bool, AppError> {
    let result = sqlx::query!(
        r#"
        SELECT
            hourly_import_limit,
            imports_this_hour,
            last_import_hour
        FROM user_ai_usage_limits
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(limits) = result {
        let now = chrono::Utc::now();
        let last_hour = limits.last_import_hour.unwrap_or_else(|| {
            chrono::DateTime::from_timestamp(0, 0).unwrap().naive_utc().and_utc()
        });

        // Reset counter if hour has passed
        if now.signed_duration_since(last_hour).num_hours() >= 1 {
            return Ok(true); // New hour, limit resets
        }

        // Check if within limit
        let imports_this_hour = limits.imports_this_hour;
        let hourly_limit = limits.hourly_import_limit;

        Ok(imports_this_hour < hourly_limit)
    } else {
        // No limits set yet, allow
        Ok(true)
    }
}

/// Increment hourly usage counter
async fn increment_hourly_usage(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    let now = chrono::Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO user_ai_usage_limits (user_id, imports_this_hour, last_import_hour)
        VALUES ($1, 1, $2)
        ON CONFLICT (user_id) DO UPDATE
        SET
            imports_this_hour = CASE
                WHEN EXTRACT(EPOCH FROM ($2 - user_ai_usage_limits.last_import_hour)) >= 3600 THEN 1
                ELSE user_ai_usage_limits.imports_this_hour + 1
            END,
            last_import_hour = $2,
            updated_at = NOW()
        "#,
        user_id,
        now
    )
    .execute(pool)
    .await?;

    Ok(())
}

// ============================================================================
// GLOBAL RATE LIMITING (IP-based) has been moved to ip_rate_limiter.rs
// This uses a custom implementation instead of tower_governor for better
// compatibility with Axum 0.7 and more control over rate limiting logic
// ============================================================================

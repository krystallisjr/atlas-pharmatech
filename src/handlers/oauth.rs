//! OAuth/OIDC HTTP Handlers
//!
//! Provides endpoints for OAuth 2.0 authentication flow.
//! Integrates with existing JWT authentication system.

use axum::{
    extract::{Path, Query, State, ConnectInfo},
    response::{IntoResponse, Redirect, Response},
    http::header,
    Json,
    Extension,
};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::{
    config::{AppConfig, oauth::{OAuthConfig, OAuthProvider, OAuthProvidersInfo}},
    services::{OAuthService, OAuthUserInfo},
    middleware::{error_handling::{Result, AppError}, Claims, JwtService},
    repositories::UserRepository,
    models::user::UserResponse,
};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct OAuthStartResponse {
    pub auth_url: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct OAuthLoginResponse {
    pub token: String,
    pub user: UserResponse,
    pub is_new_user: bool,
}

#[derive(Debug, Serialize)]
pub struct OAuthLinkResponse {
    pub success: bool,
    pub provider: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct OAuthErrorResponse {
    pub error: String,
    pub error_description: Option<String>,
    pub provider: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Get list of enabled OAuth providers
/// GET /api/auth/oauth/providers
pub async fn get_oauth_providers(
    State(_config): State<AppConfig>,
) -> Result<Json<OAuthProvidersInfo>> {
    let oauth_config = OAuthConfig::from_env()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("OAuth config error: {}", e)))?;

    Ok(Json(oauth_config.get_providers_info()))
}

/// Start OAuth flow - redirect to provider
/// GET /api/auth/oauth/:provider
pub async fn oauth_start(
    State(config): State<AppConfig>,
    Path(provider_name): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
) -> Result<Response> {
    // Parse provider
    let provider = OAuthProvider::from_str(&provider_name)
        .map_err(|_| AppError::BadRequest(format!("Unknown OAuth provider: {}", provider_name)))?;

    // Get OAuth config and service
    let oauth_config = OAuthConfig::from_env()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("OAuth config error: {}", e)))?;

    let oauth_service = OAuthService::new(config.database_pool.clone(), oauth_config.clone());

    // Get client info for security logging
    let ip_address = get_client_ip(&headers, &addr);
    let user_agent = headers.get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    // Generate authorization URL
    let auth_response = oauth_service.generate_auth_url(
        provider,
        Some(ip_address),
        user_agent,
        None, // Not linking to existing user
    ).await?;

    tracing::info!(
        provider = provider.as_str(),
        "OAuth flow started, redirecting to provider"
    );

    // Redirect to OAuth provider
    Ok(Redirect::temporary(&auth_response.auth_url).into_response())
}

/// OAuth callback - exchange code for tokens
/// GET /api/auth/oauth/:provider/callback
pub async fn oauth_callback(
    State(config): State<AppConfig>,
    Path(provider_name): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
) -> Result<Response> {
    // Check for OAuth error from provider
    if let Some(error) = &query.error {
        tracing::warn!(
            provider = %provider_name,
            error = %error,
            description = ?query.error_description,
            "OAuth provider returned error"
        );

        let oauth_config = OAuthConfig::from_env().unwrap_or_default();
        let encoded_error = utf8_percent_encode(error, NON_ALPHANUMERIC).to_string();
        let error_url = format!(
            "{}?error={}&provider={}",
            oauth_config.frontend_error_url,
            encoded_error,
            provider_name
        );
        return Ok(Redirect::temporary(&error_url).into_response());
    }

    // Parse provider
    let provider = OAuthProvider::from_str(&provider_name)
        .map_err(|_| AppError::BadRequest(format!("Unknown OAuth provider: {}", provider_name)))?;

    // Get OAuth config and service
    let oauth_config = OAuthConfig::from_env()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("OAuth config error: {}", e)))?;

    let oauth_service = OAuthService::new(config.database_pool.clone(), oauth_config.clone());

    // Get client info
    let ip_address = get_client_ip(&headers, &addr);
    let user_agent = headers.get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    // Exchange code for user info
    let oauth_user = match oauth_service.exchange_code(
        provider,
        &query.code,
        &query.state,
        Some(ip_address.clone()),
        user_agent.clone(),
    ).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!(
                provider = provider.as_str(),
                error = %e,
                "OAuth code exchange failed"
            );
            let error_url = format!(
                "{}?error=exchange_failed&provider={}",
                oauth_config.frontend_error_url,
                provider_name
            );
            return Ok(Redirect::temporary(&error_url).into_response());
        }
    };

    // Find or create user
    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;
    let (user, is_new_user) = find_or_create_oauth_user(&user_repo, &oauth_user).await?;

    // Generate JWT using existing JwtService
    let jwt_service = JwtService::new(&config.jwt_secret);
    let token = jwt_service.generate_token(
        user.id,
        &user.email,
        &user.company_name,
        user.is_verified,
        user.role.clone(),
    ).map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to generate token: {}", e)))?;

    tracing::info!(
        provider = provider.as_str(),
        user_id = %user.id,
        email = %user.email,
        is_new_user = is_new_user,
        "OAuth login successful"
    );

    // Build success redirect with token
    let encoded_token = utf8_percent_encode(&token, NON_ALPHANUMERIC).to_string();
    let success_url = format!(
        "{}?token={}&new_user={}",
        oauth_config.frontend_success_url,
        encoded_token,
        is_new_user
    );

    // Also set httpOnly cookie for security
    let cookie_value = format!(
        "auth_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=86400",
        token
    );

    Ok((
        [(header::SET_COOKIE, cookie_value)],
        Redirect::temporary(&success_url)
    ).into_response())
}

/// Link OAuth provider to existing account (requires auth)
/// POST /api/auth/oauth/link/:provider
pub async fn oauth_link_start(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(provider_name): Path<String>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: axum::http::HeaderMap,
) -> Result<Json<OAuthStartResponse>> {
    let provider = OAuthProvider::from_str(&provider_name)
        .map_err(|_| AppError::BadRequest(format!("Unknown OAuth provider: {}", provider_name)))?;

    let oauth_config = OAuthConfig::from_env()
        .map_err(|e| AppError::Internal(anyhow::anyhow!("OAuth config error: {}", e)))?;

    let oauth_service = OAuthService::new(config.database_pool.clone(), oauth_config);

    let ip_address = get_client_ip(&headers, &addr);
    let user_agent = headers.get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    // Generate auth URL with linking_user_id
    let auth_response = oauth_service.generate_auth_url(
        provider,
        Some(ip_address),
        user_agent,
        Some(claims.user_id), // Link to current user
    ).await?;

    Ok(Json(OAuthStartResponse {
        auth_url: auth_response.auth_url,
        state: auth_response.state,
    }))
}

/// Unlink OAuth provider from account (requires auth)
/// POST /api/auth/oauth/unlink/:provider
pub async fn oauth_unlink(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(provider_name): Path<String>,
) -> Result<Json<OAuthLinkResponse>> {
    let provider = OAuthProvider::from_str(&provider_name)
        .map_err(|_| AppError::BadRequest(format!("Unknown OAuth provider: {}", provider_name)))?;

    let user_repo = UserRepository::new(config.database_pool.clone(), &config.encryption_key)?;

    // Check if user has password before unlinking (can't remove all auth methods)
    let user = user_repo.find_by_id(claims.user_id).await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    if user.password_hash.is_empty() {
        return Err(AppError::BadRequest(
            "Cannot unlink OAuth - no password set. Set a password first.".to_string()
        ));
    }

    // Unlink OAuth
    unlink_oauth_from_user(&config.database_pool, claims.user_id, provider).await?;

    tracing::info!(
        user_id = %claims.user_id,
        provider = provider.as_str(),
        "OAuth provider unlinked from account"
    );

    Ok(Json(OAuthLinkResponse {
        success: true,
        provider: provider_name,
        message: "OAuth provider unlinked successfully".to_string(),
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Find existing user or create new one from OAuth info
async fn find_or_create_oauth_user(
    user_repo: &UserRepository,
    oauth_user: &OAuthUserInfo,
) -> Result<(crate::models::user::User, bool)> {
    use sqlx::query_as;

    // First, try to find by OAuth provider ID
    let existing_by_provider: Option<crate::models::user::User> = query_as(
        r#"
        SELECT id, email, password_hash, company_name, contact_person, phone, address,
               license_number, is_verified, created_at, updated_at, role as "role: _"
        FROM users
        WHERE oauth_provider = $1 AND oauth_provider_id = $2
        "#
    )
    .bind(&oauth_user.provider)
    .bind(&oauth_user.provider_id)
    .fetch_optional(user_repo.pool())
    .await?;

    if let Some(user) = existing_by_provider {
        // Update last login time
        sqlx::query(
            "UPDATE users SET oauth_last_login_at = NOW() WHERE id = $1"
        )
        .bind(user.id)
        .execute(user_repo.pool())
        .await?;

        return Ok((user, false));
    }

    // Try to find by email (for account linking)
    let existing_by_email = user_repo.find_by_email(&oauth_user.email).await?;

    if let Some(user) = existing_by_email {
        // Link OAuth to existing account
        sqlx::query(
            r#"
            UPDATE users SET
                oauth_provider = $1,
                oauth_provider_id = $2,
                oauth_email = $3,
                oauth_name = $4,
                oauth_avatar_url = $5,
                oauth_linked_at = NOW(),
                oauth_last_login_at = NOW()
            WHERE id = $6
            "#
        )
        .bind(&oauth_user.provider)
        .bind(&oauth_user.provider_id)
        .bind(&oauth_user.email)
        .bind(&oauth_user.name)
        .bind(&oauth_user.avatar_url)
        .bind(user.id)
        .execute(user_repo.pool())
        .await?;

        return Ok((user, false));
    }

    // Create new user
    let new_user_id = Uuid::new_v4();
    let company_name = oauth_user.name.clone().unwrap_or_else(|| oauth_user.email.split('@').next().unwrap_or("User").to_string());
    let contact_person = oauth_user.name.clone().unwrap_or_else(|| "OAuth User".to_string());

    sqlx::query(
        r#"
        INSERT INTO users (
            id, email, company_name, contact_person, is_verified, role,
            oauth_provider, oauth_provider_id, oauth_email, oauth_name,
            oauth_avatar_url, oauth_linked_at, oauth_last_login_at
        ) VALUES (
            $1, $2, $3, $4, true, 'user',
            $5, $6, $7, $8, $9, NOW(), NOW()
        )
        "#
    )
    .bind(new_user_id)
    .bind(&oauth_user.email)
    .bind(&company_name)
    .bind(&contact_person)
    .bind(&oauth_user.provider)
    .bind(&oauth_user.provider_id)
    .bind(&oauth_user.email)
    .bind(&oauth_user.name)
    .bind(&oauth_user.avatar_url)
    .execute(user_repo.pool())
    .await?;

    // Fetch the created user
    let user = user_repo.find_by_id(new_user_id).await?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Failed to fetch created user")))?;

    Ok((user, true))
}

/// Unlink OAuth from user account
async fn unlink_oauth_from_user(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    provider: OAuthProvider,
) -> Result<()> {
    let result = sqlx::query(
        r#"
        UPDATE users SET
            oauth_provider = NULL,
            oauth_provider_id = NULL,
            oauth_email = NULL,
            oauth_name = NULL,
            oauth_avatar_url = NULL,
            oauth_access_token_encrypted = NULL,
            oauth_refresh_token_encrypted = NULL,
            oauth_token_expires_at = NULL
        WHERE id = $1 AND oauth_provider = $2
        "#
    )
    .bind(user_id)
    .bind(provider.as_str())
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::BadRequest("OAuth provider not linked to this account".to_string()));
    }

    Ok(())
}

/// Extract client IP from headers or socket address
fn get_client_ip(headers: &axum::http::HeaderMap, addr: &SocketAddr) -> String {
    // Check X-Forwarded-For header (for reverse proxy)
    headers.get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        // Check X-Real-IP header
        .or_else(|| {
            headers.get("X-Real-IP")
                .and_then(|h| h.to_str().ok())
                .map(String::from)
        })
        // Fall back to socket address
        .unwrap_or_else(|| addr.ip().to_string())
}

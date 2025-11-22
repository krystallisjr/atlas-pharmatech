//! Production-Ready OAuth/OIDC Service
//!
//! Handles OAuth 2.0 and OpenID Connect authentication flows for Google, GitHub, and Microsoft.
//! Integrates with existing JWT authentication system.

use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, query, query_as, Row};
use uuid::Uuid;
use thiserror::Error;

use crate::config::oauth::{OAuthConfig, OAuthProvider, ProviderConfig};
use crate::middleware::error_handling::{Result, AppError};
use crate::services::EncryptionService;

/// OAuth service errors
#[derive(Debug, Error)]
pub enum OAuthError {
    #[error("Provider not configured: {0}")]
    ProviderNotConfigured(String),

    #[error("Provider disabled: {0}")]
    ProviderDisabled(String),

    #[error("Invalid state parameter")]
    InvalidState,

    #[error("State expired")]
    StateExpired,

    #[error("State already used")]
    StateAlreadyUsed,

    #[error("Failed to exchange authorization code: {0}")]
    TokenExchangeFailed(String),

    #[error("Failed to fetch user info: {0}")]
    UserInfoFailed(String),

    #[error("Email not provided by OAuth provider")]
    EmailNotProvided,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("Invalid response from provider: {0}")]
    InvalidResponse(String),
}

impl From<OAuthError> for AppError {
    fn from(err: OAuthError) -> Self {
        match err {
            OAuthError::InvalidState | OAuthError::StateExpired | OAuthError::StateAlreadyUsed => {
                AppError::BadRequest(err.to_string())
            }
            OAuthError::ProviderNotConfigured(_) | OAuthError::ProviderDisabled(_) => {
                AppError::BadRequest(err.to_string())
            }
            OAuthError::EmailNotProvided => {
                AppError::BadRequest("Email is required for authentication".to_string())
            }
            _ => AppError::Internal(anyhow::anyhow!("{}", err)),
        }
    }
}

/// User information retrieved from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthUserInfo {
    pub provider: String,
    pub provider_id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
}

/// OAuth state stored in database
#[derive(Debug, Clone)]
pub struct OAuthState {
    pub id: Uuid,
    pub state: String,
    pub nonce: String,
    pub pkce_code_verifier: Option<String>,
    pub provider: String,
    pub redirect_uri: Option<String>,
    pub linking_user_id: Option<Uuid>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// OAuth audit log entry
#[derive(Debug, Clone, Serialize)]
pub struct OAuthAuditEntry {
    pub user_id: Option<Uuid>,
    pub provider: String,
    pub event_type: String,
    pub event_details: serde_json::Value,
    pub oauth_provider_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Response from authorization URL generation
#[derive(Debug, Clone, Serialize)]
pub struct AuthorizationResponse {
    pub auth_url: String,
    pub state: String,
}

/// OAuth Service
pub struct OAuthService {
    pool: PgPool,
    config: OAuthConfig,
    http_client: reqwest::Client,
    encryption_service: Option<EncryptionService>,
}

impl OAuthService {
    /// Create a new OAuth service
    pub fn new(pool: PgPool, config: OAuthConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            pool,
            config,
            http_client,
            encryption_service: None,
        }
    }

    /// Create with encryption service for token storage
    pub fn with_encryption(mut self, encryption_key: &str) -> Self {
        self.encryption_service = EncryptionService::new(encryption_key).ok();
        self
    }

    /// Generate authorization URL for a provider
    pub async fn generate_auth_url(
        &self,
        provider: OAuthProvider,
        ip_address: Option<String>,
        user_agent: Option<String>,
        linking_user_id: Option<Uuid>,
    ) -> Result<AuthorizationResponse> {
        // Get provider config
        let provider_config = self.config.get_provider(provider)
            .ok_or_else(|| OAuthError::ProviderNotConfigured(provider.to_string()))?;

        if !provider_config.enabled {
            return Err(OAuthError::ProviderDisabled(provider.to_string()).into());
        }

        // Generate cryptographically secure state and nonce
        let state = generate_secure_token(32);
        let nonce = generate_secure_token(32);

        // Generate PKCE code verifier if required
        let pkce_code_verifier = if self.config.require_pkce {
            Some(generate_secure_token(64))
        } else {
            None
        };

        // Store state in database
        let expires_at = Utc::now() + chrono::Duration::seconds(self.config.state_ttl_seconds as i64);

        query(
            r#"
            INSERT INTO oauth_states (
                state, nonce, pkce_code_verifier, provider, redirect_uri,
                linking_user_id, ip_address, user_agent, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7::inet, $8, $9)
            "#
        )
        .bind(&state)
        .bind(&nonce)
        .bind(&pkce_code_verifier)
        .bind(provider.as_str())
        .bind(self.config.callback_url(provider))
        .bind(linking_user_id)
        .bind(&ip_address)
        .bind(&user_agent)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;

        // Build authorization URL
        let auth_url = self.build_auth_url(provider, provider_config, &state, &nonce, &pkce_code_verifier)?;

        tracing::info!(
            provider = provider.as_str(),
            ip = ?ip_address,
            "OAuth authorization initiated"
        );

        Ok(AuthorizationResponse { auth_url, state })
    }

    /// Build the authorization URL with all required parameters
    fn build_auth_url(
        &self,
        provider: OAuthProvider,
        config: &ProviderConfig,
        state: &str,
        nonce: &str,
        pkce_verifier: &Option<String>,
    ) -> Result<String> {
        let mut url = url::Url::parse(provider.authorization_endpoint())
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Invalid auth endpoint: {}", e)))?;

        // Add query parameters
        {
            let mut params = url.query_pairs_mut();
            params.append_pair("client_id", &config.client_id);
            params.append_pair("redirect_uri", &self.config.callback_url(provider));
            params.append_pair("response_type", "code");
            params.append_pair("state", state);
            params.append_pair("scope", &config.scopes.join(" "));

            // Add nonce for OIDC providers
            if provider.supports_oidc() {
                params.append_pair("nonce", nonce);
            }

            // Add PKCE challenge if enabled
            if let Some(verifier) = pkce_verifier {
                let challenge = generate_pkce_challenge(verifier);
                params.append_pair("code_challenge", &challenge);
                params.append_pair("code_challenge_method", "S256");
            }

            // Provider-specific parameters
            match provider {
                OAuthProvider::Google => {
                    params.append_pair("access_type", "offline"); // Get refresh token
                    params.append_pair("prompt", "consent"); // Force consent to get refresh token
                }
                OAuthProvider::GitHub => {
                    params.append_pair("allow_signup", "true");
                }
                OAuthProvider::Microsoft => {
                    params.append_pair("response_mode", "query");
                }
            }
        }

        Ok(url.to_string())
    }

    /// Exchange authorization code for tokens and user info
    pub async fn exchange_code(
        &self,
        provider: OAuthProvider,
        code: &str,
        state: &str,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<OAuthUserInfo> {
        let start_time = Instant::now();

        // Validate and consume state
        let oauth_state = self.validate_and_consume_state(state).await?;

        // Verify provider matches
        if oauth_state.provider != provider.as_str() {
            return Err(OAuthError::InvalidState.into());
        }

        // Get provider config
        let provider_config = self.config.get_provider(provider)
            .ok_or_else(|| OAuthError::ProviderNotConfigured(provider.to_string()))?;

        // Exchange code for tokens
        let tokens = self.exchange_code_for_tokens(
            provider,
            provider_config,
            code,
            oauth_state.pkce_code_verifier.as_deref(),
        ).await?;

        // Fetch user info from provider
        let user_info = self.fetch_user_info(provider, &tokens).await?;

        // Log success
        self.log_audit(OAuthAuditEntry {
            user_id: None,
            provider: provider.to_string(),
            event_type: "code_exchange".to_string(),
            event_details: serde_json::json!({
                "duration_ms": start_time.elapsed().as_millis(),
                "has_refresh_token": tokens.refresh_token.is_some(),
            }),
            oauth_provider_id: Some(user_info.provider_id.clone()),
            ip_address,
            user_agent,
            success: true,
            error_message: None,
        }).await;

        tracing::info!(
            provider = provider.as_str(),
            provider_id = %user_info.provider_id,
            email = %user_info.email,
            duration_ms = start_time.elapsed().as_millis(),
            "OAuth code exchange successful"
        );

        Ok(user_info)
    }

    /// Validate state and mark as used
    async fn validate_and_consume_state(&self, state: &str) -> Result<OAuthState> {
        // Find and lock the state record
        let row = query(
            r#"
            UPDATE oauth_states
            SET used_at = NOW()
            WHERE state = $1
              AND used_at IS NULL
              AND expires_at > NOW()
            RETURNING id, state, nonce, pkce_code_verifier, provider, redirect_uri,
                      linking_user_id, ip_address::text, user_agent, created_at, expires_at, used_at
            "#
        )
        .bind(state)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => Ok(OAuthState {
                id: row.get("id"),
                state: row.get("state"),
                nonce: row.get("nonce"),
                pkce_code_verifier: row.get("pkce_code_verifier"),
                provider: row.get("provider"),
                redirect_uri: row.get("redirect_uri"),
                linking_user_id: row.get("linking_user_id"),
                ip_address: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                created_at: row.get("created_at"),
                expires_at: row.get("expires_at"),
                used_at: row.get("used_at"),
            }),
            None => {
                // Check if state exists but is expired or used
                let exists = query("SELECT 1 FROM oauth_states WHERE state = $1")
                    .bind(state)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;

                if exists.is_some() {
                    Err(OAuthError::StateAlreadyUsed.into())
                } else {
                    Err(OAuthError::InvalidState.into())
                }
            }
        }
    }

    /// Exchange authorization code for tokens
    async fn exchange_code_for_tokens(
        &self,
        provider: OAuthProvider,
        config: &ProviderConfig,
        code: &str,
        pkce_verifier: Option<&str>,
    ) -> Result<TokenResponse> {
        // Store callback URL to extend its lifetime
        let redirect_uri = self.config.callback_url(provider);

        let mut form_params = vec![
            ("client_id", config.client_id.as_str()),
            ("client_secret", config.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
        ];

        if let Some(verifier) = pkce_verifier {
            form_params.push(("code_verifier", verifier));
        }

        let mut request = self.http_client
            .post(provider.token_endpoint())
            .form(&form_params);

        // GitHub requires Accept header
        if provider == OAuthProvider::GitHub {
            request = request.header("Accept", "application/json");
        }

        let response = request.send().await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!(
                provider = provider.as_str(),
                status = %status,
                body = %body,
                "Token exchange failed"
            );
            return Err(OAuthError::TokenExchangeFailed(format!("Status: {}", status)).into());
        }

        let tokens: TokenResponse = response.json().await
            .map_err(|e| OAuthError::InvalidResponse(e.to_string()))?;

        Ok(tokens)
    }

    /// Fetch user info from provider
    async fn fetch_user_info(
        &self,
        provider: OAuthProvider,
        tokens: &TokenResponse,
    ) -> Result<OAuthUserInfo> {
        let response = self.http_client
            .get(provider.userinfo_endpoint())
            .bearer_auth(&tokens.access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(OAuthError::UserInfoFailed(format!("Status: {}", status)).into());
        }

        // Parse provider-specific response
        let user_info = match provider {
            OAuthProvider::Google => {
                let info: GoogleUserInfo = response.json().await
                    .map_err(|e| OAuthError::InvalidResponse(e.to_string()))?;
                OAuthUserInfo {
                    provider: provider.to_string(),
                    provider_id: info.sub,
                    email: info.email.ok_or(OAuthError::EmailNotProvided)?,
                    name: info.name,
                    avatar_url: info.picture,
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.clone(),
                    token_expires_at: tokens.expires_in.map(|e| Utc::now() + chrono::Duration::seconds(e as i64)),
                }
            }
            OAuthProvider::GitHub => {
                let info: GitHubUserInfo = response.json().await
                    .map_err(|e| OAuthError::InvalidResponse(e.to_string()))?;

                // GitHub requires separate API call for email if not public
                let email = if let Some(email) = info.email {
                    email
                } else {
                    self.fetch_github_email(&tokens.access_token).await?
                };

                OAuthUserInfo {
                    provider: provider.to_string(),
                    provider_id: info.id.to_string(),
                    email,
                    name: info.name.or(Some(info.login)),
                    avatar_url: info.avatar_url,
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.clone(),
                    token_expires_at: None, // GitHub tokens don't expire
                }
            }
            OAuthProvider::Microsoft => {
                let info: MicrosoftUserInfo = response.json().await
                    .map_err(|e| OAuthError::InvalidResponse(e.to_string()))?;
                OAuthUserInfo {
                    provider: provider.to_string(),
                    provider_id: info.sub,
                    email: info.email.ok_or(OAuthError::EmailNotProvided)?,
                    name: info.name,
                    avatar_url: None, // Microsoft Graph requires separate call for photo
                    access_token: tokens.access_token.clone(),
                    refresh_token: tokens.refresh_token.clone(),
                    token_expires_at: tokens.expires_in.map(|e| Utc::now() + chrono::Duration::seconds(e as i64)),
                }
            }
        };

        Ok(user_info)
    }

    /// Fetch GitHub user's primary email
    async fn fetch_github_email(&self, access_token: &str) -> Result<String> {
        let response = self.http_client
            .get("https://api.github.com/user/emails")
            .bearer_auth(access_token)
            .header("Accept", "application/json")
            .header("User-Agent", "Atlas-Pharma")
            .send()
            .await
            .map_err(|e| OAuthError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(OAuthError::EmailNotProvided.into());
        }

        let emails: Vec<GitHubEmail> = response.json().await
            .map_err(|e| OAuthError::InvalidResponse(e.to_string()))?;

        // Find primary verified email
        emails.into_iter()
            .find(|e| e.primary && e.verified)
            .map(|e| e.email)
            .ok_or_else(|| OAuthError::EmailNotProvided.into())
    }

    /// Log an audit entry
    async fn log_audit(&self, entry: OAuthAuditEntry) {
        let result = query(
            r#"
            INSERT INTO oauth_audit_log (
                user_id, provider, event_type, event_details, oauth_provider_id,
                ip_address, user_agent, success, error_message
            ) VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8, $9)
            "#
        )
        .bind(entry.user_id)
        .bind(&entry.provider)
        .bind(&entry.event_type)
        .bind(&entry.event_details)
        .bind(&entry.oauth_provider_id)
        .bind(&entry.ip_address)
        .bind(&entry.user_agent)
        .bind(entry.success)
        .bind(&entry.error_message)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::error!("Failed to log OAuth audit entry: {}", e);
        }
    }

    /// Clean up expired OAuth states
    pub async fn cleanup_expired_states(&self) -> Result<i64> {
        let result = query("SELECT cleanup_expired_oauth_states()")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Cleanup failed: {}", e)))?;

        let deleted: i32 = result.get(0);
        Ok(deleted as i64)
    }

    /// Get enabled providers info for frontend
    pub fn get_providers_info(&self) -> crate::config::oauth::OAuthProvidersInfo {
        self.config.get_providers_info()
    }
}

// ============================================================================
// Token Response Structures
// ============================================================================

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: Option<String>,
    expires_in: Option<i64>,
    refresh_token: Option<String>,
    scope: Option<String>,
    id_token: Option<String>,
}

// ============================================================================
// Provider-Specific User Info Structures
// ============================================================================

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubUserInfo {
    id: i64,
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[derive(Debug, Deserialize)]
struct MicrosoftUserInfo {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    preferred_username: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Generate a cryptographically secure random token
fn generate_secure_token(length: usize) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..length).map(|_| rng.gen()).collect();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Generate PKCE code challenge from verifier (S256 method)
fn generate_pkce_challenge(verifier: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(&hash)
}

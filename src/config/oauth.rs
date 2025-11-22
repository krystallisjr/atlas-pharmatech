//! OAuth/OIDC Configuration Module
//!
//! Provides production-ready configuration for OAuth 2.0 and OpenID Connect providers.
//! Supports Google, GitHub, and Microsoft with automatic OIDC discovery.

use std::env;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// OAuth configuration errors
#[derive(Debug, Error)]
pub enum OAuthConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid OAuth provider: {0}")]
    InvalidProvider(String),

    #[error("OAuth is disabled for provider: {0}")]
    ProviderDisabled(String),

    #[error("Invalid redirect URI: {0}")]
    InvalidRedirectUri(String),
}

/// Supported OAuth providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProvider {
    Google,
    GitHub,
    Microsoft,
}

impl OAuthProvider {
    /// Parse provider from string
    pub fn from_str(s: &str) -> Result<Self, OAuthConfigError> {
        match s.to_lowercase().as_str() {
            "google" => Ok(Self::Google),
            "github" => Ok(Self::GitHub),
            "microsoft" => Ok(Self::Microsoft),
            _ => Err(OAuthConfigError::InvalidProvider(s.to_string())),
        }
    }

    /// Get provider name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::GitHub => "github",
            Self::Microsoft => "microsoft",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::GitHub => "GitHub",
            Self::Microsoft => "Microsoft",
        }
    }

    /// Get OIDC issuer URL (for discovery)
    pub fn issuer_url(&self) -> Option<&'static str> {
        match self {
            Self::Google => Some("https://accounts.google.com"),
            Self::GitHub => None, // GitHub doesn't support OIDC discovery
            Self::Microsoft => Some("https://login.microsoftonline.com/common/v2.0"),
        }
    }

    /// Get authorization endpoint (for non-OIDC providers)
    pub fn authorization_endpoint(&self) -> &'static str {
        match self {
            Self::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            Self::GitHub => "https://github.com/login/oauth/authorize",
            Self::Microsoft => "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
        }
    }

    /// Get token endpoint
    pub fn token_endpoint(&self) -> &'static str {
        match self {
            Self::Google => "https://oauth2.googleapis.com/token",
            Self::GitHub => "https://github.com/login/oauth/access_token",
            Self::Microsoft => "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        }
    }

    /// Get user info endpoint
    pub fn userinfo_endpoint(&self) -> &'static str {
        match self {
            Self::Google => "https://openidconnect.googleapis.com/v1/userinfo",
            Self::GitHub => "https://api.github.com/user",
            Self::Microsoft => "https://graph.microsoft.com/oidc/userinfo",
        }
    }

    /// Get default scopes for this provider
    pub fn default_scopes(&self) -> Vec<&'static str> {
        match self {
            Self::Google => vec!["openid", "email", "profile"],
            Self::GitHub => vec!["read:user", "user:email"],
            Self::Microsoft => vec!["openid", "email", "profile"],
        }
    }

    /// Whether this provider supports OIDC (id_token)
    pub fn supports_oidc(&self) -> bool {
        match self {
            Self::Google => true,
            Self::GitHub => false, // GitHub uses OAuth 2.0, not OIDC
            Self::Microsoft => true,
        }
    }
}

impl std::fmt::Display for OAuthProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Configuration for a single OAuth provider
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider: OAuthProvider,
    pub client_id: String,
    pub client_secret: String,
    pub enabled: bool,
    pub scopes: Vec<String>,
}

impl ProviderConfig {
    /// Load provider configuration from environment variables
    pub fn from_env(provider: OAuthProvider) -> Result<Option<Self>, OAuthConfigError> {
        let prefix = provider.as_str().to_uppercase();

        // Check if provider is enabled (default: enabled if credentials exist)
        let enabled_key = format!("{}_OAUTH_ENABLED", prefix);
        let client_id_key = format!("{}_CLIENT_ID", prefix);
        let client_secret_key = format!("{}_CLIENT_SECRET", prefix);
        let scopes_key = format!("{}_OAUTH_SCOPES", prefix);

        // Get client credentials
        let client_id = env::var(&client_id_key).ok();
        let client_secret = env::var(&client_secret_key).ok();

        // If no credentials, provider is not configured
        if client_id.is_none() || client_secret.is_none() {
            return Ok(None);
        }

        // Check explicit enable/disable
        let enabled = env::var(&enabled_key)
            .map(|v| v.to_lowercase() != "false" && v != "0")
            .unwrap_or(true); // Default enabled if credentials exist

        // Parse custom scopes or use defaults
        let scopes = env::var(&scopes_key)
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_else(|_| {
                provider.default_scopes()
                    .into_iter()
                    .map(String::from)
                    .collect()
            });

        Ok(Some(Self {
            provider,
            client_id: client_id.unwrap(),
            client_secret: client_secret.unwrap(),
            enabled,
            scopes,
        }))
    }
}

/// Main OAuth configuration holding all provider configs
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    /// Base URL for OAuth callbacks (e.g., "https://api.example.com")
    pub redirect_base_url: String,

    /// Callback path template (provider name will be substituted)
    pub callback_path: String,

    /// Provider configurations
    pub google: Option<ProviderConfig>,
    pub github: Option<ProviderConfig>,
    pub microsoft: Option<ProviderConfig>,

    /// Security settings
    pub state_ttl_seconds: u64,
    pub require_pkce: bool,

    /// Frontend redirect after successful auth
    pub frontend_success_url: String,
    pub frontend_error_url: String,
}

impl OAuthConfig {
    /// Load OAuth configuration from environment variables
    pub fn from_env() -> Result<Self, OAuthConfigError> {
        let redirect_base_url = env::var("OAUTH_REDIRECT_BASE_URL")
            .or_else(|_| env::var("API_BASE_URL"))
            .unwrap_or_else(|_| "http://localhost:8443".to_string());

        let callback_path = env::var("OAUTH_CALLBACK_PATH")
            .unwrap_or_else(|_| "/api/auth/oauth/{provider}/callback".to_string());

        let frontend_success_url = env::var("OAUTH_SUCCESS_REDIRECT")
            .unwrap_or_else(|_| "http://localhost:3000/auth/oauth/callback".to_string());

        let frontend_error_url = env::var("OAUTH_ERROR_REDIRECT")
            .unwrap_or_else(|_| "http://localhost:3000/login?error=oauth_failed".to_string());

        let state_ttl_seconds = env::var("OAUTH_STATE_TTL_SECONDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600); // 10 minutes default

        let require_pkce = env::var("OAUTH_REQUIRE_PKCE")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(true); // PKCE enabled by default

        Ok(Self {
            redirect_base_url,
            callback_path,
            google: ProviderConfig::from_env(OAuthProvider::Google)?,
            github: ProviderConfig::from_env(OAuthProvider::GitHub)?,
            microsoft: ProviderConfig::from_env(OAuthProvider::Microsoft)?,
            state_ttl_seconds,
            require_pkce,
            frontend_success_url,
            frontend_error_url,
        })
    }

    /// Get configuration for a specific provider
    pub fn get_provider(&self, provider: OAuthProvider) -> Option<&ProviderConfig> {
        match provider {
            OAuthProvider::Google => self.google.as_ref(),
            OAuthProvider::GitHub => self.github.as_ref(),
            OAuthProvider::Microsoft => self.microsoft.as_ref(),
        }
    }

    /// Get the callback URL for a provider
    pub fn callback_url(&self, provider: OAuthProvider) -> String {
        let path = self.callback_path.replace("{provider}", provider.as_str());
        format!("{}{}", self.redirect_base_url, path)
    }

    /// Check if a provider is enabled and configured
    pub fn is_provider_enabled(&self, provider: OAuthProvider) -> bool {
        self.get_provider(provider)
            .map(|p| p.enabled)
            .unwrap_or(false)
    }

    /// Get list of enabled providers
    pub fn enabled_providers(&self) -> Vec<OAuthProvider> {
        let mut providers = Vec::new();
        if self.is_provider_enabled(OAuthProvider::Google) {
            providers.push(OAuthProvider::Google);
        }
        if self.is_provider_enabled(OAuthProvider::GitHub) {
            providers.push(OAuthProvider::GitHub);
        }
        if self.is_provider_enabled(OAuthProvider::Microsoft) {
            providers.push(OAuthProvider::Microsoft);
        }
        providers
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), OAuthConfigError> {
        // Validate redirect base URL
        if !self.redirect_base_url.starts_with("http://") && !self.redirect_base_url.starts_with("https://") {
            return Err(OAuthConfigError::InvalidRedirectUri(
                "redirect_base_url must start with http:// or https://".to_string()
            ));
        }

        // Warn if using HTTP in production
        if self.redirect_base_url.starts_with("http://") && !self.redirect_base_url.contains("localhost") {
            tracing::warn!(
                "⚠️  OAuth redirect URL uses HTTP ({}). Use HTTPS in production!",
                self.redirect_base_url
            );
        }

        Ok(())
    }
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self::from_env().unwrap_or_else(|_| Self {
            redirect_base_url: "http://localhost:8443".to_string(),
            callback_path: "/api/auth/oauth/{provider}/callback".to_string(),
            google: None,
            github: None,
            microsoft: None,
            state_ttl_seconds: 600,
            require_pkce: true,
            frontend_success_url: "http://localhost:3000/auth/oauth/callback".to_string(),
            frontend_error_url: "http://localhost:3000/login?error=oauth_failed".to_string(),
        })
    }
}

/// Information about enabled OAuth providers (for frontend)
#[derive(Debug, Clone, Serialize)]
pub struct OAuthProvidersInfo {
    pub providers: Vec<ProviderInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderInfo {
    pub name: String,
    pub display_name: String,
    pub auth_url: String,
}

impl OAuthConfig {
    /// Get provider info for frontend
    pub fn get_providers_info(&self) -> OAuthProvidersInfo {
        let providers = self.enabled_providers()
            .into_iter()
            .map(|p| ProviderInfo {
                name: p.as_str().to_string(),
                display_name: p.display_name().to_string(),
                auth_url: format!("/api/auth/oauth/{}", p.as_str()),
            })
            .collect();

        OAuthProvidersInfo { providers }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_str() {
        assert_eq!(OAuthProvider::from_str("google").unwrap(), OAuthProvider::Google);
        assert_eq!(OAuthProvider::from_str("GITHUB").unwrap(), OAuthProvider::GitHub);
        assert_eq!(OAuthProvider::from_str("Microsoft").unwrap(), OAuthProvider::Microsoft);
        assert!(OAuthProvider::from_str("invalid").is_err());
    }

    #[test]
    fn test_provider_scopes() {
        assert!(OAuthProvider::Google.default_scopes().contains(&"openid"));
        assert!(OAuthProvider::GitHub.default_scopes().contains(&"user:email"));
    }
}

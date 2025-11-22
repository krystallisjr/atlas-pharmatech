use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use crate::config::AppConfig;
use crate::models::user::UserRole;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub user_id: Uuid,
    pub email: String,
    pub company_name: String,
    pub is_verified: bool,
    pub role: UserRole,
    pub exp: usize,
    pub iat: usize,
    pub jti: String,  // JWT ID for token blacklist
}

impl Claims {
    /// Check if user has admin privileges
    pub fn is_admin(&self) -> bool {
        self.role.is_admin()
    }

    /// Check if user has superadmin privileges
    pub fn is_superadmin(&self) -> bool {
        self.role.is_superadmin()
    }
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }

    pub fn generate_token(&self, user_id: Uuid, email: &str, company_name: &str, is_verified: bool, role: UserRole) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        // Admin session timeout: 2 hours (more secure)
        // Regular user: 24 hours
        let exp = if role.is_admin() {
            now + 2 * 60 * 60  // 2 hours for admins
        } else {
            now + 24 * 60 * 60  // 24 hours for regular users
        };

        let claims = Claims {
            sub: user_id.to_string(),
            user_id,
            email: email.to_string(),
            company_name: company_name.to_string(),
            is_verified,
            role,
            exp,
            iat: now,
            jti: Uuid::new_v4().to_string(),  // Unique token ID for blacklist tracking
        };

        encode(&Header::default(), &claims, &self.encoding_key)
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
    }

    pub fn extract_token_from_header(auth_header: &str) -> Option<&str> {
        if auth_header.starts_with("Bearer ") {
            Some(&auth_header[7..])
        } else {
            None
        }
    }
}

pub async fn auth_middleware(
    State(config): State<AppConfig>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let jwt_service = JwtService::new(&config.jwt_secret);

    // Get blacklist service from extensions
    use crate::services::TokenBlacklistService;
    use std::sync::Arc;
    let blacklist = request
        .extensions()
        .get::<Arc<TokenBlacklistService>>()
        .cloned();

    // SECURITY: Try to extract token from cookie first (preferred, more secure)
    let cookie_jar = CookieJar::from_headers(request.headers());
    let token = if let Some(cookie) = cookie_jar.get("auth_token") {
        Some(cookie.value())
    } else {
        // Fallback: check Authorization header for backwards compatibility
        request
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(JwtService::extract_token_from_header)
    };

    if let Some(token) = token {
        match jwt_service.validate_token(token) {
            Ok(claims) => {
                // 🔒 SECURITY: Check if token is blacklisted (logout/revoked)
                if let Some(ref blacklist) = blacklist {
                    if blacklist.is_blacklisted(&claims.jti) {
                        tracing::warn!("Blocked blacklisted token for user {}", claims.user_id);
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }

                request.extensions_mut().insert(claims);
                return Ok(next.run(request).await);
            }
            Err(_) => return Err(StatusCode::UNAUTHORIZED),
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

pub fn create_optional_auth_middleware() -> impl Fn(State<AppConfig>, Request, Next) -> Pin<Box<dyn Future<Output = Result<Response, StatusCode>> + Send>> {
    move |State(config): State<AppConfig>, mut request: Request, next: Next| {
        Box::pin(async move {
            let auth_header = request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok());

            if let Some(auth_header) = auth_header {
                if let Some(token) = JwtService::extract_token_from_header(auth_header) {
                    let jwt_service = JwtService::new(&config.jwt_secret);
                    
                    if let Ok(claims) = jwt_service.validate_token(token) {
                        request.extensions_mut().insert(claims);
                    }
                }
            }

            Ok(next.run(request).await)
        })
    }
}

use std::pin::Pin;
use futures::Future;
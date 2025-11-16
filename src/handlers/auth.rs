use axum::{
    extract::{State, Extension},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use validator::Validate;
use time::Duration;
use crate::{
    models::user::{CreateUserRequest, LoginRequest, UserResponse},
    services::AuthService,
    middleware::{Claims, error_handling::{Result, AppError}},
    config::AppConfig,
};

/// Create a secure httpOnly cookie for JWT token
///
/// Security features:
/// - HttpOnly: Prevents XSS attacks (JavaScript cannot access)
/// - Secure: Only sent over HTTPS (production)
/// - SameSite::Strict: Prevents CSRF attacks
/// - Max-Age: 24 hours
fn create_auth_cookie(token: String, is_production: bool) -> Cookie<'static> {
    Cookie::build(("auth_token", token))
        .path("/")
        .max_age(Duration::days(1))  // 24 hours
        .http_only(true)  // XSS protection
        .secure(is_production)  // Only HTTPS in production
        .same_site(SameSite::Strict)  // CSRF protection
        .build()
}

/// Create a logout cookie (expires immediately)
fn create_logout_cookie() -> Cookie<'static> {
    Cookie::build(("auth_token", ""))
        .path("/")
        .max_age(Duration::ZERO)
        .http_only(true)
        .build()
}

pub async fn register(
    State(config): State<AppConfig>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Response> {
    request.validate()
        .map_err(|e| AppError::Validation(e))?;

    let auth_service = AuthService::new(
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        &config.jwt_secret,
    );

    let (user, token) = auth_service.register(request).await?;

    // Check if TLS is enabled (production mode)
    let is_production = std::env::var("TLS_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let cookie = create_auth_cookie(token.clone(), is_production);

    // Return user data + set cookie
    let mut response = Json(user).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

pub async fn login(
    State(config): State<AppConfig>,
    Extension(audit): Extension<std::sync::Arc<crate::services::ComprehensiveAuditService>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Response> {
    request.validate()
        .map_err(|e| AppError::Validation(e))?;

    let email = request.email.clone();
    let ip_address = Some(addr.ip());
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let auth_service = AuthService::new(
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        &config.jwt_secret,
    );

    // Attempt login
    let login_result = auth_service.login(request).await;

    match login_result {
        Ok((user, token)) => {
            // 🔐 PRODUCTION MFA CHECK: If user has MFA enabled, require verification before issuing full auth token
            let mfa_service = crate::services::MfaTotpService::new(
                config.database_pool.clone(),
                &config.encryption_key,
                "Atlas Pharma".to_string(),
            )?;

            let mfa_enabled = mfa_service.is_mfa_enabled(user.id).await?;

            if mfa_enabled {
                // Check if this is a trusted device
                let device_fingerprint = format!(
                    "{}-{}",
                    addr.ip(),
                    user_agent.as_deref().unwrap_or("unknown")
                );
                let is_trusted = mfa_service.is_trusted_device(user.id, &device_fingerprint).await?;

                if !is_trusted {
                    // MFA required - return special response WITHOUT setting auth cookie
                    tracing::info!("🔐 MFA verification required for user: {}", email);

                    // Return MFA required response
                    return Ok(Json(serde_json::json!({
                        "mfa_required": true,
                        "email": email,
                        "user_id": user.id,
                    })).into_response());
                }

                // Trusted device - proceed with normal login
                tracing::info!("✅ Trusted device detected for user: {}", email);
            }

            // 📋 AUDIT: Log successful login
            let _ = audit.log_login_success(
                user.id,
                &email,
                ip_address,
                user_agent.clone(),
            ).await;

            // Check if TLS is enabled (production mode)
            let is_production = std::env::var("TLS_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false);

            let cookie = create_auth_cookie(token.clone(), is_production);

            // Return user data + token + set cookie
            let response_body = serde_json::json!({
                "user": user,
                "token": token,
            });
            let mut response = Json(response_body).into_response();
            response.headers_mut().insert(
                header::SET_COOKIE,
                cookie.to_string().parse().unwrap(),
            );

            Ok(response)
        }
        Err(e) => {
            // 📋 AUDIT: Log failed login attempt
            let reason = match &e {
                AppError::Unauthorized => "invalid_credentials",
                _ => "system_error",
            };
            let _ = audit.log_login_failed(
                &email,
                reason,
                ip_address,
                user_agent,
            ).await;

            Err(e)
        }
    }
}

pub async fn get_profile(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserResponse>> {
    let auth_service = AuthService::new(
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        &config.jwt_secret,
    );

    let user = auth_service.get_user(claims.user_id).await?;
    Ok(Json(user))
}

pub async fn update_profile(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<crate::models::user::UpdateUserRequest>,
) -> Result<Json<UserResponse>> {
    request.validate()
        .map_err(|e| AppError::Validation(e))?;

    let auth_service = AuthService::new(
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        &config.jwt_secret,
    );

    let user = auth_service.update_user(claims.user_id, request).await?;
    Ok(Json(user))
}

pub async fn delete_account(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<StatusCode> {
    let auth_service = AuthService::new(
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        &config.jwt_secret,
    );

    auth_service.delete_user(claims.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn refresh_token(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Response> {
    let auth_service = AuthService::new(
        crate::repositories::UserRepository::new(config.database_pool.clone(), &config.encryption_key)?,
        &config.jwt_secret,
    );

    let user = auth_service.get_user(claims.user_id).await?;
    let new_token = auth_service.generate_token(
        user.id,
        &user.email,
        &user.company_name,
        user.is_verified,
    )?;

    // Check if TLS is enabled (production mode)
    let is_production = std::env::var("TLS_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let cookie = create_auth_cookie(new_token, is_production);

    // Return success message + refresh cookie
    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

pub async fn logout(
    Extension(claims): Extension<Claims>,
    Extension(blacklist): Extension<std::sync::Arc<crate::services::TokenBlacklistService>>,
) -> Result<Response> {
    // 🔒 SECURITY: Add token to blacklist so it can't be reused
    let exp_timestamp = claims.exp as u64;
    let exp_duration = std::time::Duration::from_secs(exp_timestamp.saturating_sub(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ));
    let expires_at = std::time::Instant::now() + exp_duration;

    blacklist.blacklist_token(
        claims.jti.clone(),
        claims.user_id,
        expires_at,
        "user_logout".to_string(),
    );

    let cookie = create_logout_cookie();

    let mut response = StatusCode::OK.into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}
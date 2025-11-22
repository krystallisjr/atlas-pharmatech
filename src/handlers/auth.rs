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

    // 🔒 SECURITY: Add CSRF token for protection against cross-site request forgery
    crate::middleware::csrf_protection::add_csrf_token_to_response(&mut response);

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
                    // 🔒 SECURITY: Sanitize email for log injection prevention
                    tracing::info!("🔐 MFA verification required for user: {}",
                        crate::utils::log_sanitizer::sanitize_for_log(&email));

                    // Return MFA required response
                    return Ok(Json(serde_json::json!({
                        "mfa_required": true,
                        "email": email,
                        "user_id": user.id,
                    })).into_response());
                }

                // Trusted device - proceed with normal login
                // 🔒 SECURITY: Sanitize email for log injection prevention
                tracing::info!("✅ Trusted device detected for user: {}",
                    crate::utils::log_sanitizer::sanitize_for_log(&email));
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

            // 🔒 SECURITY: Add CSRF token for protection against cross-site request forgery
            crate::middleware::csrf_protection::add_csrf_token_to_response(&mut response);

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
        user.role,
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
/// Change user password with session invalidation
///
/// 🔒 SECURITY: Password change invalidates ALL existing sessions
///
/// **Security Features:**
/// 1. Requires current password verification
/// 2. Validates new password strength
/// 3. Invalidates ALL tokens (logout from all devices)
/// 4. Issues new token for current session
/// 5. Comprehensive audit logging
///
/// **Compliance:**
/// - NIST SP 800-63B (Password requirements)
/// - PCI DSS Requirement 8.2.4 (Password change)
/// - HIPAA §164.308(a)(5) (Access management)
///
pub async fn change_password(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Extension(blacklist): Extension<std::sync::Arc<crate::services::TokenBlacklistService>>,
    Extension(audit): Extension<std::sync::Arc<crate::services::ComprehensiveAuditService>>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Json(request): Json<serde_json::Value>,
) -> Result<Response> {
    // Extract passwords from request
    let current_password = request.get("current_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("current_password required".to_string()))?;

    let new_password = request.get("new_password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("new_password required".to_string()))?;

    // 🔒 SECURITY: Validate new password strength
    if new_password.len() < 8 {
        return Err(AppError::BadRequest("Password must be at least 8 characters".to_string()));
    }

    // Get user from database
    let user_repo = crate::repositories::UserRepository::new(
        config.database_pool.clone(),
        &config.encryption_key
    )?;

    let user = user_repo
        .find_by_id(claims.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    // Verify current password
    let is_valid = bcrypt::verify(current_password, &user.password_hash)?;
    if !is_valid {
        // 📋 AUDIT: Log failed password change attempt
        let _ = audit.log(crate::services::comprehensive_audit_service::AuditLogEntry {
            event_type: "password_change_failed".to_string(),
            event_category: crate::services::comprehensive_audit_service::EventCategory::Security,
            severity: crate::services::comprehensive_audit_service::Severity::Warning,
            actor_user_id: Some(claims.user_id),
            actor_type: "user".to_string(),
            resource_type: Some("user_password".to_string()),
            action: "change_password".to_string(),
            action_result: crate::services::comprehensive_audit_service::ActionResult::Failure,
            event_data: serde_json::json!({
                "reason": "invalid_current_password"
            }),
            ..Default::default()
        }).await;

        return Err(AppError::Unauthorized);
    }

    // 🔒 SECURITY: Hash new password with bcrypt
    let new_password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)?;

    // Update password in database
    sqlx::query!(
        "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2",
        new_password_hash,
        claims.user_id
    )
    .execute(&config.database_pool)
    .await?;

    // 🔒 SECURITY: Invalidate ALL tokens for this user (logout from all devices)
    // This prevents stolen sessions from remaining valid after password change
    blacklist.revoke_user_tokens(
        claims.user_id,
        "password_change".to_string(),
    );

    tracing::info!(
        "✅ Password changed for user: {} (all sessions invalidated)",
        claims.user_id
    );

    // 📋 AUDIT: Log successful password change
    let _ = audit.log(crate::services::comprehensive_audit_service::AuditLogEntry {
        event_type: "password_change_success".to_string(),
        event_category: crate::services::comprehensive_audit_service::EventCategory::Security,
        severity: crate::services::comprehensive_audit_service::Severity::Info,
        actor_user_id: Some(claims.user_id),
        actor_type: "user".to_string(),
        resource_type: Some("user_password".to_string()),
        action: "change_password".to_string(),
        action_result: crate::services::comprehensive_audit_service::ActionResult::Success,
        event_data: serde_json::json!({
            "all_sessions_invalidated": true
        }),
        ..Default::default()
    }).await;

    // Generate new token for current session
    let auth_service = AuthService::new(user_repo, &config.jwt_secret);
    let new_token = auth_service.generate_token(
        user.id,
        &user.email,
        &user.company_name,
        user.is_verified,
        user.role.clone(),
    )?;

    // Set new auth cookie
    let is_production = std::env::var("TLS_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .parse()
        .unwrap_or(false);

    let cookie = create_auth_cookie(new_token.clone(), is_production);

    let response_body = serde_json::json!({
        "message": "Password changed successfully. All other sessions have been logged out.",
        "token": new_token,
    });

    let mut response = Json(response_body).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    // 🔒 SECURITY: Add new CSRF token
    crate::middleware::csrf_protection::add_csrf_token_to_response(&mut response);

    Ok(response)
}

/// MFA/TOTP API Handlers
/// Production-grade multi-factor authentication endpoints

use axum::{
    extract::{State, Path},
    Extension,
    Json,
};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::{
    config::AppConfig,
    middleware::{error_handling::Result, Claims},
    services::MfaTotpService,
};

// ============================================================================
// REQUEST/RESPONSE TYPES
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct StartEnrollmentRequest {
    /// User's password for re-authentication
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct StartEnrollmentResponse {
    /// Base32-encoded TOTP secret
    pub secret: String,
    /// Base64-encoded QR code PNG image
    pub qr_code: String,
    /// Backup codes (show once, user must save)
    pub backup_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteEnrollmentRequest {
    /// TOTP code from authenticator app (to verify setup)
    pub totp_code: String,
    /// Base32-encoded secret from start_enrollment
    pub secret: String,
    /// Backup codes from start_enrollment
    pub backup_codes: Vec<String>,
    /// Optional device name
    pub device_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyMfaRequest {
    /// TOTP code or backup code
    pub code: String,
    /// Whether to trust this device
    pub trust_device: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct VerifyMfaResponse {
    pub success: bool,
    pub trusted_device_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct DisableMfaRequest {
    /// User's password for re-authentication
    pub password: String,
    /// Backup code or TOTP code for verification
    pub mfa_code: String,
}

#[derive(Debug, Serialize)]
pub struct MfaStatusResponse {
    pub mfa_enabled: bool,
    pub enrolled_at: Option<String>,
    pub backup_codes_remaining: i32,
    pub trusted_devices_count: i32,
}

#[derive(Debug, Serialize)]
pub struct TrustedDevice {
    pub id: Uuid,
    pub device_name: Option<String>,
    pub device_type: Option<String>,
    pub trusted_at: String,
    pub expires_at: String,
    pub last_used_at: String,
}

// ============================================================================
// HANDLERS
// ============================================================================

/// GET /api/mfa/status
/// Get user's MFA status
pub async fn get_mfa_status(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<MfaStatusResponse>> {
    let mfa_service = MfaTotpService::new(
        config.database_pool.clone(),
        &config.encryption_key,
        "Atlas Pharma".to_string(),
    )?;

    let status = mfa_service.get_user_mfa_status(claims.user_id).await?;

    // Get trusted devices count
    let trusted_devices_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM mfa_trusted_devices WHERE user_id = $1 AND is_active = TRUE"
    )
    .bind(claims.user_id)
    .fetch_one(&config.database_pool)
    .await? as i32;

    Ok(Json(MfaStatusResponse {
        mfa_enabled: status.enabled,
        enrolled_at: status.enrolled_at.map(|dt| dt.to_rfc3339()),
        backup_codes_remaining: status.backup_codes_remaining,
        trusted_devices_count,
    }))
}

/// POST /api/mfa/enroll/start
/// Start MFA enrollment (generate secret and QR code)
pub async fn start_enrollment(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<StartEnrollmentRequest>,
) -> Result<Json<StartEnrollmentResponse>> {
    // Re-authenticate user with password
    let user_repo = crate::repositories::UserRepository::new(
        config.database_pool.clone(),
        &config.encryption_key,
    )?;

    let user = user_repo.find_by_id(claims.user_id).await?
        .ok_or(crate::middleware::error_handling::AppError::NotFound("User not found".to_string()))?;

    let is_valid = bcrypt::verify(&request.password, &user.password_hash)?;
    if !is_valid {
        return Err(crate::middleware::error_handling::AppError::Unauthorized);
    }

    // Generate TOTP secret and QR code
    let mfa_service = MfaTotpService::new(
        config.database_pool.clone(),
        &config.encryption_key,
        "Atlas Pharma".to_string(),
    )?;

    let (secret, qr_code) = mfa_service.generate_totp_secret(&claims.email)?;

    // Generate backup codes
    let backup_codes = mfa_service.generate_backup_codes();

    tracing::info!("🔐 MFA enrollment started for user: {}", claims.user_id);

    Ok(Json(StartEnrollmentResponse {
        secret,
        qr_code,
        backup_codes,
    }))
}

/// POST /api/mfa/enroll/complete
/// Complete MFA enrollment (verify TOTP code and save)
pub async fn complete_enrollment(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Json(request): Json<CompleteEnrollmentRequest>,
) -> Result<Json<serde_json::Value>> {
    let mfa_service = MfaTotpService::new(
        config.database_pool.clone(),
        &config.encryption_key,
        "Atlas Pharma".to_string(),
    )?;

    // Verify TOTP code
    let is_valid = mfa_service.verify_totp_code(&request.secret, &request.totp_code)?;
    if !is_valid {
        return Err(crate::middleware::error_handling::AppError::BadRequest(
            "Invalid TOTP code. Please try again.".to_string()
        ));
    }

    // Enroll user
    mfa_service.enroll_user_mfa(
        claims.user_id,
        &request.secret,
        request.backup_codes,
        request.device_name.clone(),
        Some(addr.ip().to_string()),
    ).await?;

    tracing::info!("✅ MFA enrollment completed for user: {}", claims.user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "MFA successfully enabled"
    })))
}

/// POST /api/mfa/verify
/// Verify MFA code during login
pub async fn verify_mfa(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    headers: axum::http::HeaderMap,
    Json(request): Json<VerifyMfaRequest>,
) -> Result<Json<VerifyMfaResponse>> {
    let mfa_service = MfaTotpService::new(
        config.database_pool.clone(),
        &config.encryption_key,
        "Atlas Pharma".to_string(),
    )?;

    let ip_address = Some(addr.ip().to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    // Check rate limiting
    let within_limit = mfa_service.check_rate_limit(claims.user_id).await?;
    if !within_limit {
        mfa_service.log_verification_attempt(
            claims.user_id,
            "totp",
            "rate_limited",
            ip_address.clone(),
            user_agent.clone(),
        ).await?;

        return Err(crate::middleware::error_handling::AppError::TooManyRequests(
            "Too many failed attempts. Please try again later.".to_string()
        ));
    }

    // Get user's TOTP secret
    let secret = mfa_service.get_user_totp_secret(claims.user_id).await?
        .ok_or(crate::middleware::error_handling::AppError::BadRequest(
            "MFA not enabled for this user".to_string()
        ))?;

    // Try TOTP verification first
    let is_totp_valid = mfa_service.verify_totp_code(&secret, &request.code)?;

    let mut trusted_device_id = None;

    if is_totp_valid {
        // Log successful verification
        mfa_service.log_verification_attempt(
            claims.user_id,
            "totp",
            "success",
            ip_address.clone(),
            user_agent.clone(),
        ).await?;

        // Add trusted device if requested
        if request.trust_device.unwrap_or(false) {
            let device_fingerprint = format!("{}-{}", addr.ip(), user_agent.as_deref().unwrap_or("unknown"));
            let device_id = mfa_service.add_trusted_device(
                claims.user_id,
                device_fingerprint,
                user_agent.clone(),
                None,
                ip_address.clone(),
                user_agent.clone(),
                30, // 30 days
            ).await?;
            trusted_device_id = Some(device_id);
        }

        Ok(Json(VerifyMfaResponse {
            success: true,
            trusted_device_id,
        }))
    } else {
        // Try backup code
        let is_backup_valid = mfa_service.verify_and_consume_backup_code(
            claims.user_id,
            &request.code,
        ).await?;

        if is_backup_valid {
            mfa_service.log_verification_attempt(
                claims.user_id,
                "backup_code",
                "success",
                ip_address,
                user_agent,
            ).await?;

            Ok(Json(VerifyMfaResponse {
                success: true,
                trusted_device_id: None,
            }))
        } else {
            // Log failed attempt
            mfa_service.log_verification_attempt(
                claims.user_id,
                "totp",
                "invalid_code",
                ip_address,
                user_agent,
            ).await?;

            Err(crate::middleware::error_handling::AppError::BadRequest(
                "Invalid MFA code".to_string()
            ))
        }
    }
}

/// POST /api/mfa/disable
/// Disable MFA for user
pub async fn disable_mfa(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    Json(request): Json<DisableMfaRequest>,
) -> Result<Json<serde_json::Value>> {
    // Re-authenticate with password
    let user_repo = crate::repositories::UserRepository::new(
        config.database_pool.clone(),
        &config.encryption_key,
    )?;

    let user = user_repo.find_by_id(claims.user_id).await?
        .ok_or(crate::middleware::error_handling::AppError::NotFound("User not found".to_string()))?;

    let is_valid = bcrypt::verify(&request.password, &user.password_hash)?;
    if !is_valid {
        return Err(crate::middleware::error_handling::AppError::Unauthorized);
    }

    // Verify MFA code
    let mfa_service = MfaTotpService::new(
        config.database_pool.clone(),
        &config.encryption_key,
        "Atlas Pharma".to_string(),
    )?;

    let secret = mfa_service.get_user_totp_secret(claims.user_id).await?
        .ok_or(crate::middleware::error_handling::AppError::BadRequest(
            "MFA not enabled".to_string()
        ))?;

    let is_totp_valid = mfa_service.verify_totp_code(&secret, &request.mfa_code)?;
    let is_backup_valid = mfa_service.verify_and_consume_backup_code(
        claims.user_id,
        &request.mfa_code,
    ).await?;

    if !is_totp_valid && !is_backup_valid {
        return Err(crate::middleware::error_handling::AppError::BadRequest(
            "Invalid MFA code".to_string()
        ));
    }

    // Disable MFA
    mfa_service.disable_user_mfa(
        claims.user_id,
        Some(addr.ip().to_string()),
    ).await?;

    tracing::warn!("⚠️  MFA disabled for user: {}", claims.user_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "MFA successfully disabled"
    })))
}

/// GET /api/mfa/trusted-devices
/// Get list of trusted devices
pub async fn get_trusted_devices(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<TrustedDevice>>> {
    let devices = sqlx::query!(
        r#"
        SELECT
            id,
            device_name,
            device_type,
            trusted_at,
            expires_at,
            last_used_at
        FROM mfa_trusted_devices
        WHERE user_id = $1 AND is_active = TRUE
        ORDER BY last_used_at DESC
        "#,
        claims.user_id
    )
    .fetch_all(&config.database_pool)
    .await?
    .into_iter()
    .map(|d| TrustedDevice {
        id: d.id,
        device_name: d.device_name,
        device_type: d.device_type,
        trusted_at: d.trusted_at.to_rfc3339(),
        expires_at: d.expires_at.to_rfc3339(),
        last_used_at: d.last_used_at.to_rfc3339(),
    })
    .collect();

    Ok(Json(devices))
}

/// DELETE /api/mfa/trusted-devices/:id
/// Revoke a trusted device
pub async fn revoke_trusted_device(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(device_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let mfa_service = MfaTotpService::new(
        config.database_pool.clone(),
        &config.encryption_key,
        "Atlas Pharma".to_string(),
    )?;

    mfa_service.revoke_trusted_device(claims.user_id, device_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Device revoked successfully"
    })))
}

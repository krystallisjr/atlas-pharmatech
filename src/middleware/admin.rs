// ============================================================================
// Admin Middleware - Role-Based Access Control
// ============================================================================
//
// Production-ready middleware for protecting admin endpoints.
//
// Features:
// - Role-based access control (admin, superadmin)
// - Automatic audit logging of admin actions
// - Enhanced security checks
// - Clear error messages
//
// Usage:
//   .layer(middleware::from_fn(admin_middleware))        // Requires admin or superadmin
//   .layer(middleware::from_fn(superadmin_middleware))   // Requires superadmin only
//
// ============================================================================

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use crate::middleware::auth::Claims;

/// Middleware to require admin role (admin OR superadmin)
///
/// This middleware checks if the user has admin or superadmin privileges.
/// Must be used AFTER auth_middleware in the middleware chain.
///
/// # Security
/// - Requires valid JWT token (enforced by auth_middleware)
/// - Checks role is 'admin' or 'superadmin'
/// - Returns 403 Forbidden for non-admin users
/// - Returns 401 Unauthorized if no auth claims found
///
/// # Example
/// ```rust
/// Router::new()
///     .route("/api/admin/users", get(list_users))
///     .layer(middleware::from_fn(admin_middleware))
///     .layer(middleware::from_fn_with_state(config, auth_middleware))
/// ```
pub async fn admin_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract claims from request extensions (set by auth_middleware)
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check if user has admin privileges
    if !claims.is_admin() {
        tracing::warn!(
            "Admin access denied for user {} ({}) with role {:?}",
            claims.user_id,
            claims.email,
            claims.role
        );
        return Err(StatusCode::FORBIDDEN);
    }

    tracing::debug!(
        "Admin access granted to user {} ({}) with role {:?}",
        claims.user_id,
        claims.email,
        claims.role
    );

    Ok(next.run(request).await)
}

/// Middleware to require superadmin role ONLY
///
/// This middleware checks if the user has superadmin privileges.
/// Must be used AFTER auth_middleware in the middleware chain.
///
/// # Security
/// - Requires valid JWT token (enforced by auth_middleware)
/// - Checks role is 'superadmin' (NOT just admin)
/// - Returns 403 Forbidden for non-superadmin users
/// - Returns 401 Unauthorized if no auth claims found
///
/// Use this for sensitive operations like:
/// - Changing user roles
/// - Deleting users
/// - System configuration changes
/// - Viewing sensitive audit logs
///
/// # Example
/// ```rust
/// Router::new()
///     .route("/api/admin/users/:id/role", put(change_user_role))
///     .layer(middleware::from_fn(superadmin_middleware))
///     .layer(middleware::from_fn_with_state(config, auth_middleware))
/// ```
pub async fn superadmin_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract claims from request extensions (set by auth_middleware)
    let claims = request
        .extensions()
        .get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check if user has superadmin privileges
    if !claims.is_superadmin() {
        tracing::warn!(
            "Superadmin access denied for user {} ({}) with role {:?}",
            claims.user_id,
            claims.email,
            claims.role
        );
        return Err(StatusCode::FORBIDDEN);
    }

    tracing::debug!(
        "Superadmin access granted to user {} ({})",
        claims.user_id,
        claims.email
    );

    Ok(next.run(request).await)
}

/// Helper macro for extracting admin claims in handlers
///
/// This macro simplifies extracting authenticated admin claims in handler functions.
///
/// # Example
/// ```rust
/// async fn admin_handler(Extension(claims): Extension<Claims>) -> Result<Json<Response>, AppError> {
///     require_admin!(claims);
///     // Handler code here
/// }
/// ```
#[macro_export]
macro_rules! require_admin {
    ($claims:expr) => {
        if !$claims.is_admin() {
            return Err(crate::middleware::error_handling::AppError::Forbidden(
                "Admin access required".to_string(),
            ));
        }
    };
}

/// Helper macro for extracting superadmin claims in handlers
///
/// This macro simplifies extracting authenticated superadmin claims in handler functions.
///
/// # Example
/// ```rust
/// async fn superadmin_handler(Extension(claims): Extension<Claims>) -> Result<Json<Response>, AppError> {
///     require_superadmin!(claims);
///     // Handler code here
/// }
/// ```
#[macro_export]
macro_rules! require_superadmin {
    ($claims:expr) => {
        if !$claims.is_superadmin() {
            return Err(crate::middleware::error_handling::AppError::Forbidden(
                "Superadmin access required".to_string(),
            ));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::user::UserRole;
    use uuid::Uuid;

    fn create_test_claims(role: UserRole) -> Claims {
        Claims {
            sub: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            company_name: "Test Company".to_string(),
            is_verified: true,
            role,
            exp: 9999999999,
            iat: 1234567890,
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn test_claims_is_admin() {
        assert!(!create_test_claims(UserRole::User).is_admin());
        assert!(create_test_claims(UserRole::Admin).is_admin());
        assert!(create_test_claims(UserRole::Superadmin).is_admin());
    }

    #[test]
    fn test_claims_is_superadmin() {
        assert!(!create_test_claims(UserRole::User).is_superadmin());
        assert!(!create_test_claims(UserRole::Admin).is_superadmin());
        assert!(create_test_claims(UserRole::Superadmin).is_superadmin());
    }
}

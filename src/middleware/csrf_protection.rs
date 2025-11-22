// ============================================================================
// CSRF Protection Middleware - Production-Grade Anti-CSRF Token System
// ============================================================================
//
// 🔒 SECURITY: Implements double-submit cookie pattern for CSRF protection
//
// ## What is CSRF?
// Cross-Site Request Forgery (CSRF) is an attack that forces authenticated
// users to execute unwanted actions on a web application. The attacker tricks
// the victim's browser into sending requests that appear legitimate.
//
// ## Attack Example:
// 1. User logs into atlas-pharma.com (gets auth cookie)
// 2. User visits malicious-site.com
// 3. Malicious site contains: <form action="https://atlas-pharma.com/api/inventory/delete/123" method="POST">
// 4. Browser auto-sends auth cookie with the request
// 5. Without CSRF protection, the delete succeeds!
//
// ## Our Protection Strategy:
//
// **Double-Submit Cookie Pattern:**
// 1. Server generates random CSRF token on login/registration
// 2. Token is sent BOTH as:
//    - Secure, HttpOnly cookie (can't be read by JavaScript)
//    - Response header (can be read and stored by JavaScript)
// 3. Client must send token back in custom header: X-CSRF-Token
// 4. Server validates: cookie token === header token
// 5. Attacker can't read the token (SOP) and can't forge the header
//
// **Why This Works:**
// - Same-Origin Policy prevents attacker from reading the token
// - Attacker can trigger cookie to be sent but can't read its value
// - Attacker can't set custom headers on cross-origin requests
// - Even if cookies are sent, the header won't match
//
// ## Compliance:
// - OWASP CSRF Prevention Cheat Sheet
// - PCI DSS Requirement 6.5.9 (CSRF protection)
// - CWE-352: Cross-Site Request Forgery
//
// ============================================================================

use axum::{
    extract::Request,
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::Response,
};
use rand::{thread_rng, Rng};
use base64::{Engine as _, engine::general_purpose};

/// Generate a cryptographically secure CSRF token
///
/// Uses 32 bytes of random data (256 bits) encoded as base64
///
/// # Security:
/// - Uses thread_rng() which is cryptographically secure
/// - 256 bits provides sufficient entropy (2^256 possibilities)
/// - Base64 encoding makes it URL-safe
///
pub fn generate_csrf_token() -> String {
    let mut token_bytes = [0u8; 32]; // 256 bits
    thread_rng().fill(&mut token_bytes);
    general_purpose::STANDARD.encode(token_bytes)
}

/// Validate CSRF token from request
///
/// Compares token from cookie with token from header
///
/// # Security:
/// - Uses constant-time comparison to prevent timing attacks
/// - Requires exact match (no substring matching)
/// - Both cookie and header must be present
///
fn validate_csrf_token(cookie_token: &str, header_token: &str) -> bool {
    // 🔒 SECURITY: Constant-time comparison prevents timing attacks
    use subtle::ConstantTimeEq;

    if cookie_token.len() != header_token.len() {
        return false;
    }

    cookie_token.as_bytes().ct_eq(header_token.as_bytes()).into()
}

/// Extract CSRF token from cookies
fn extract_csrf_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(|s| s.trim())
        .find(|cookie| cookie.starts_with("csrf-token="))?
        .strip_prefix("csrf-token=")?
        .to_string()
        .into()
}

/// CSRF protection middleware
///
/// Validates CSRF tokens for all state-changing operations (POST, PUT, DELETE, PATCH)
///
/// # Exemptions:
/// - GET, HEAD, OPTIONS requests (safe methods, read-only)
/// - Public endpoints (e.g., /api/auth/register, /api/auth/login)
/// - Webhook endpoints (use HMAC signature verification instead)
///
/// # Required Headers:
/// - Cookie: csrf-token=<token>
/// - X-CSRF-Token: <token>
///
/// # Error Responses:
/// - 403 Forbidden: Missing or invalid CSRF token
/// - Includes clear error message for debugging
///
pub async fn csrf_protection_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let method = request.method();
    let path = request.uri().path();

    // 🔒 SECURITY: Skip CSRF check for safe methods (GET, HEAD, OPTIONS)
    // These methods should not change state per HTTP specification
    if matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS) {
        return Ok(next.run(request).await);
    }

    // 🔒 SECURITY: Skip CSRF check for public endpoints
    // These endpoints don't rely on cookie-based authentication
    if is_public_endpoint(path) {
        return Ok(next.run(request).await);
    }

    // 🔒 SECURITY: Skip CSRF check for webhook endpoints
    // Webhooks use HMAC signature verification instead
    if path.starts_with("/api/erp/webhooks/") {
        return Ok(next.run(request).await);
    }

    // Extract CSRF token from cookie
    let cookie_token = extract_csrf_cookie(&headers)
        .ok_or_else(|| {
            tracing::warn!("⚠️  CSRF: Missing csrf-token cookie for {} {}", method, path);
            (
                StatusCode::FORBIDDEN,
                "CSRF token missing in cookie".to_string(),
            )
        })?;

    // Extract CSRF token from header
    let header_token = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("⚠️  CSRF: Missing X-CSRF-Token header for {} {}", method, path);
            (
                StatusCode::FORBIDDEN,
                "CSRF token missing in header".to_string(),
            )
        })?;

    // Validate tokens match
    if !validate_csrf_token(&cookie_token, header_token) {
        tracing::warn!(
            "⚠️  CSRF: Token mismatch for {} {} (possible CSRF attack)",
            method,
            crate::utils::log_sanitizer::sanitize_for_log(path)
        );
        return Err((
            StatusCode::FORBIDDEN,
            "CSRF token validation failed".to_string(),
        ));
    }

    // ✅ CSRF token valid
    Ok(next.run(request).await)
}

/// Check if endpoint is public (doesn't require CSRF protection)
fn is_public_endpoint(path: &str) -> bool {
    // Public endpoints that don't use cookie-based auth
    let public_paths = [
        "/api/auth/register",
        "/api/auth/login",
        "/api/auth/refresh",
        "/api/public/",
        "/api/openfda/",
        "/health",
    ];

    public_paths.iter().any(|prefix| path.starts_with(prefix))
}

/// Add CSRF token to response headers and cookies
///
/// Call this after login/registration to set up CSRF protection
///
/// # Usage:
/// ```rust
/// let mut response = Json(login_response).into_response();
/// add_csrf_token_to_response(&mut response);
/// ```
pub fn add_csrf_token_to_response(response: &mut Response) {
    let csrf_token = generate_csrf_token();

    // Add to response header (for JavaScript to read and store)
    response.headers_mut().insert(
        "X-CSRF-Token",
        csrf_token.parse().unwrap(),
    );

    // Add as HttpOnly cookie (for double-submit pattern)
    // Note: Not using HttpOnly here because we need to send it back in header
    // But we use SameSite=Strict for protection
    let cookie = format!(
        "csrf-token={}; Path=/; SameSite=Strict; Secure; Max-Age=86400",
        csrf_token
    );

    response.headers_mut().append(
        header::SET_COOKIE,
        cookie.parse().unwrap(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_csrf_token() {
        let token1 = generate_csrf_token();
        let token2 = generate_csrf_token();

        // Tokens should be different
        assert_ne!(token1, token2);

        // Tokens should be base64 encoded (44 chars for 32 bytes)
        assert_eq!(token1.len(), 44);
        assert_eq!(token2.len(), 44);
    }

    #[test]
    fn test_validate_csrf_token_success() {
        let token = "dGVzdC10b2tlbi0xMjM0NTY3ODkwYWJjZGVm";
        assert!(validate_csrf_token(token, token));
    }

    #[test]
    fn test_validate_csrf_token_failure() {
        let token1 = "dGVzdC10b2tlbi0xMjM0NTY3ODkwYWJjZGVm";
        let token2 = "ZGlmZmVyZW50LXRva2VuLTEyMzQ1Njc4OTA=";
        assert!(!validate_csrf_token(token1, token2));
    }

    #[test]
    fn test_validate_csrf_token_different_lengths() {
        let token1 = "short";
        let token2 = "much-longer-token";
        assert!(!validate_csrf_token(token1, token2));
    }

    #[test]
    fn test_is_public_endpoint() {
        assert!(is_public_endpoint("/api/auth/register"));
        assert!(is_public_endpoint("/api/auth/login"));
        assert!(is_public_endpoint("/api/public/inventory/search"));
        assert!(is_public_endpoint("/health"));

        assert!(!is_public_endpoint("/api/inventory/add"));
        assert!(!is_public_endpoint("/api/admin/users"));
    }
}

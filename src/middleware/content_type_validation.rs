// ============================================================================
// Content-Type Validation Middleware - Prevent Content Confusion Attacks
// ============================================================================
//
// 🔒 SECURITY: Validates Content-Type headers to prevent attacks
//
// ## Problem:
// Accepting requests with incorrect Content-Type headers can lead to:
// - Content confusion attacks
// - Parser vulnerabilities
// - Logging errors
// - Unexpected behavior
//
// ## Solution:
// Validate Content-Type header matches expected type for endpoint
//
// ## Attack Example:
// ```
// POST /api/auth/login
// Content-Type: application/xml
// Body: {"email": "test@example.com", "password": "pass"}
// ```
//
// Server might:
// - Try to parse as XML (fail)
// - Log the error with body content (leak credentials)
// - Return confusing error message
//
// ## Implementation:
// - JSON endpoints require application/json
// - Multipart endpoints require multipart/form-data
// - Reject requests with missing or invalid Content-Type
//
// ============================================================================

use axum::{
    extract::Request,
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Validate Content-Type header for state-changing requests
///
/// # Rules:
/// - GET, HEAD, OPTIONS, DELETE: No Content-Type required
/// - POST, PUT, PATCH with body: Content-Type required
/// - JSON APIs: Must be application/json
/// - Multipart: Must be multipart/form-data
///
pub async fn content_type_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let method = request.method();
    let path = request.uri().path();

    // Skip validation for methods that don't have request bodies
    if matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS | &Method::DELETE) {
        return Ok(next.run(request).await);
    }

    // For POST, PUT, PATCH - validate Content-Type
    if matches!(method, &Method::POST | &Method::PUT | &Method::PATCH) {
        let content_type = request
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok());

        match content_type {
            Some(ct) => {
                // Check if it's a valid content type for our API
                if is_valid_content_type(ct, path) {
                    Ok(next.run(request).await)
                } else {
                    tracing::warn!(
                        "⚠️  INVALID CONTENT-TYPE: method={}, path={}, content_type={}",
                        method,
                        crate::utils::log_sanitizer::sanitize_for_log(path),
                        crate::utils::log_sanitizer::sanitize_for_log(ct)
                    );

                    Err((
                        StatusCode::UNSUPPORTED_MEDIA_TYPE,
                        format!("Unsupported Content-Type: {}. Expected: application/json or multipart/form-data", ct),
                    ))
                }
            }
            None => {
                tracing::warn!(
                    "⚠️  MISSING CONTENT-TYPE: method={}, path={}",
                    method,
                    crate::utils::log_sanitizer::sanitize_for_log(path)
                );

                Err((
                    StatusCode::BAD_REQUEST,
                    "Content-Type header required for this request".to_string(),
                ))
            }
        }
    } else {
        Ok(next.run(request).await)
    }
}

/// Check if content type is valid for the given path
fn is_valid_content_type(content_type: &str, path: &str) -> bool {
    // Normalize content type (remove charset and other parameters)
    let ct_base = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_lowercase();

    // Multipart endpoints (file uploads)
    if path.contains("/upload") || path.contains("/import") {
        // Accept both multipart and JSON (some upload endpoints accept JSON)
        return ct_base.starts_with("multipart/form-data") ||
               ct_base == "application/json";
    }

    // All other endpoints should be JSON
    ct_base == "application/json"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_json_content_type() {
        assert!(is_valid_content_type("application/json", "/api/auth/login"));
        assert!(is_valid_content_type("application/json; charset=utf-8", "/api/auth/login"));
    }

    #[test]
    fn test_valid_multipart_content_type() {
        assert!(is_valid_content_type("multipart/form-data", "/api/upload"));
        assert!(is_valid_content_type("multipart/form-data; boundary=----", "/api/import"));
    }

    #[test]
    fn test_invalid_content_type() {
        assert!(!is_valid_content_type("application/xml", "/api/auth/login"));
        assert!(!is_valid_content_type("text/plain", "/api/auth/login"));
        assert!(!is_valid_content_type("text/html", "/api/auth/login"));
    }

    #[test]
    fn test_upload_accepts_both() {
        assert!(is_valid_content_type("application/json", "/api/upload"));
        assert!(is_valid_content_type("multipart/form-data", "/api/upload"));
    }
}

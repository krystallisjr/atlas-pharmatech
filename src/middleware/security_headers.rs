// ============================================================================
// Security Headers Middleware - Production-Grade HTTP Security Headers
// ============================================================================
//
// Implements comprehensive security headers to protect against:
// - Clickjacking attacks (X-Frame-Options)
// - MIME-type sniffing (X-Content-Type-Options)
// - XSS attacks (X-XSS-Protection, Content-Security-Policy)
// - Man-in-the-middle attacks (Strict-Transport-Security)
// - Information disclosure (X-Powered-By removal)
//
// Compliance: OWASP Top 10, PCI DSS, SOC 2, HIPAA
//
// ============================================================================

use axum::{
    extract::Request,
    http::{HeaderValue, header},
    middleware::Next,
    response::Response,
};

/// Production-ready security headers middleware
///
/// Adds comprehensive security headers to all responses to protect against
/// common web application vulnerabilities.
///
/// # Security Headers Applied:
///
/// 1. **X-Content-Type-Options: nosniff**
///    - Prevents MIME-type sniffing
///    - Forces browser to respect Content-Type header
///    - Mitigates drive-by download attacks
///
/// 2. **X-Frame-Options: DENY**
///    - Prevents clickjacking attacks
///    - Blocks page from being embedded in iframe/frame/object
///    - Alternative: SAMEORIGIN (allows same-origin framing)
///
/// 3. **X-XSS-Protection: 1; mode=block**
///    - Legacy XSS protection for older browsers
///    - Modern browsers use CSP instead
///    - Blocks page load if XSS detected
///
/// 4. **Strict-Transport-Security: max-age=31536000; includeSubDomains**
///    - Forces HTTPS connections for 1 year
///    - Applies to all subdomains
///    - Prevents SSL stripping attacks
///
/// 5. **Content-Security-Policy**
///    - Mitigates XSS, injection attacks, and data exfiltration
///    - Restricts resource loading to trusted sources
///    - Production-ready policy for pharmaceutical B2B platform
///
/// 6. **Referrer-Policy: strict-origin-when-cross-origin**
///    - Controls referrer information sent with requests
///    - Protects privacy and prevents information leakage
///    - Sends full URL for same-origin, origin only for cross-origin
///
/// 7. **Permissions-Policy**
///    - Controls browser features and APIs
///    - Disables unnecessary features (geolocation, microphone, camera, etc.)
///    - Reduces attack surface
///
/// 8. **X-Powered-By: (removed)**
///    - Removes server identification headers
///    - Prevents information disclosure
///    - Makes fingerprinting more difficult
///
/// # Usage:
/// ```rust
/// use axum::Router;
/// use middleware::from_fn;
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(from_fn(security_headers_middleware));
/// ```
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // 🔒 1. X-Content-Type-Options: nosniff
    // Prevents MIME-type sniffing, forces browser to respect Content-Type
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );

    // 🔒 2. X-Frame-Options: DENY
    // Prevents clickjacking by blocking iframe embedding
    headers.insert(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    );

    // 🔒 3. X-XSS-Protection: 1; mode=block
    // Legacy XSS protection for older browsers (IE, Chrome, Safari)
    headers.insert(
        header::X_XSS_PROTECTION,
        HeaderValue::from_static("1; mode=block"),
    );

    // 🔒 4. Strict-Transport-Security (HSTS)
    // Forces HTTPS for 1 year, including all subdomains
    // IMPORTANT: Only add this header when serving over HTTPS
    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // 🔒 5. Content-Security-Policy (CSP)
    // Production-ready CSP for pharmaceutical B2B platform
    //
    // Policy breakdown:
    // - default-src 'self': Only load resources from same origin by default
    // - script-src 'self' 'unsafe-inline': Allow same-origin scripts + inline scripts
    //   (Note: 'unsafe-inline' needed for Next.js. Consider using nonces in production)
    // - style-src 'self' 'unsafe-inline': Allow same-origin + inline styles
    //   (Note: 'unsafe-inline' needed for Tailwind CSS and styled components)
    // - img-src 'self' data: https:: Allow images from same-origin, data URIs, and HTTPS
    // - font-src 'self' data:: Allow fonts from same-origin and data URIs
    // - connect-src 'self': Only allow AJAX/WebSocket to same origin
    // - frame-ancestors 'none': Prevent framing (redundant with X-Frame-Options)
    // - base-uri 'self': Restrict <base> tag to same origin
    // - form-action 'self': Only allow form submissions to same origin
    // - upgrade-insecure-requests: Automatically upgrade HTTP to HTTPS
    //
    // TODO: For stricter security, consider:
    // - Removing 'unsafe-inline' and using nonces/hashes
    // - Adding specific domains instead of 'https:' wildcard
    // - Implementing CSP reporting endpoint
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(
            "default-src 'self'; \
             script-src 'self' 'unsafe-inline'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data: https:; \
             font-src 'self' data:; \
             connect-src 'self'; \
             frame-ancestors 'none'; \
             base-uri 'self'; \
             form-action 'self'; \
             upgrade-insecure-requests"
        ),
    );

    // 🔒 6. Referrer-Policy
    // Control referrer information to protect privacy
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // 🔒 7. Permissions-Policy (formerly Feature-Policy)
    // Disable unnecessary browser features to reduce attack surface
    //
    // Disabled features:
    // - geolocation: No location tracking needed
    // - microphone: No audio recording needed
    // - camera: No video/photo capture needed
    // - payment: No Payment Request API needed
    // - usb: No USB access needed
    // - magnetometer/gyroscope/accelerometer: No motion sensors needed
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static(
            "geolocation=(), \
             microphone=(), \
             camera=(), \
             payment=(), \
             usb=(), \
             magnetometer=(), \
             gyroscope=(), \
             accelerometer=()"
        ),
    );

    // 🔒 8. Remove X-Powered-By header if present
    // Prevents server fingerprinting and information disclosure
    headers.remove("x-powered-by");

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
        routing::get,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_security_headers_applied() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(axum::middleware::from_fn(security_headers_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let headers = response.headers();

        // Verify all security headers are present
        assert_eq!(
            headers.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
            "nosniff"
        );
        assert_eq!(
            headers.get(header::X_FRAME_OPTIONS).unwrap(),
            "DENY"
        );
        assert_eq!(
            headers.get(header::X_XSS_PROTECTION).unwrap(),
            "1; mode=block"
        );
        assert_eq!(
            headers.get(header::STRICT_TRANSPORT_SECURITY).unwrap(),
            "max-age=31536000; includeSubDomains"
        );
        assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
        assert_eq!(
            headers.get(header::REFERRER_POLICY).unwrap(),
            "strict-origin-when-cross-origin"
        );

        // Verify X-Powered-By is not present
        assert!(!headers.contains_key("x-powered-by"));
    }

    #[tokio::test]
    async fn test_csp_header_comprehensive() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(axum::middleware::from_fn(security_headers_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let csp = response.headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap()
            .to_str()
            .unwrap();

        // Verify CSP directives
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(csp.contains("upgrade-insecure-requests"));
    }
}

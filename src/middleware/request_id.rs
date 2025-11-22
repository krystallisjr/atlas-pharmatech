// ============================================================================
// Request ID Middleware - Distributed Request Tracking
// ============================================================================
//
// 🔒 OBSERVABILITY: Adds unique request ID to every request for tracing
//
// ## Benefits:
//
// 1. **Request Correlation**
//    - Track a single request across multiple services
//    - Correlate logs from different components
//    - Debug distributed systems issues
//
// 2. **Debugging**
//    - Quickly find all logs for a specific request
//    - Reproduce issues by request ID
//    - Share request IDs with users for support
//
// 3. **Performance Monitoring**
//    - Track request latency end-to-end
//    - Identify slow requests
//    - Build request-level metrics
//
// 4. **Audit Trail**
//    - Link audit logs to specific requests
//    - Forensic analysis of security incidents
//    - Compliance requirements (SOC 2, HIPAA)
//
// ## Header Format:
//
// Request: `X-Request-ID: <uuid>` (optional, client can provide)
// Response: `X-Request-ID: <uuid>` (always returned)
//
// If client provides X-Request-ID, we use it (for distributed tracing)
// Otherwise, we generate a new UUID v4
//
// ## Usage in Logs:
//
// ```rust
// tracing::info!(
//     request_id = %request_id,
//     "Processing user login"
// );
// ```
//
// ## Compliance:
// - SOC 2 CC7.2 (Logging and Monitoring)
// - HIPAA §164.312(b) (Audit Controls)
// - PCI DSS Requirement 10.2 (Audit Trail)
//
// ============================================================================

use axum::{
    extract::Request,
    http::header,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

/// Request ID header name (de facto standard)
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Extract or generate request ID, add to response headers
///
/// # Flow:
/// 1. Check if client provided X-Request-ID header
/// 2. If yes, validate and use it
/// 3. If no, generate new UUID v4
/// 4. Add request ID to response headers
/// 5. Log request with request ID
///
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract request ID from incoming request
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(|| Uuid::new_v4());

    // Store request ID in request extensions for use in handlers
    request.extensions_mut().insert(request_id);

    // Log incoming request with request ID
    tracing::info!(
        request_id = %request_id,
        method = %request.method(),
        uri = %request.uri(),
        "→ Incoming request"
    );

    // Process request
    let mut response = next.run(request).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        header::HeaderName::from_static(REQUEST_ID_HEADER),
        request_id.to_string().parse().unwrap(),
    );

    // Log outgoing response with request ID
    tracing::info!(
        request_id = %request_id,
        status = %response.status(),
        "← Outgoing response"
    );

    response
}

/// Extract request ID from request extensions
///
/// # Usage:
/// ```rust
/// pub async fn my_handler(
///     Extension(request_id): Extension<Uuid>,
/// ) -> Result<String> {
///     tracing::info!(request_id = %request_id, "Handler called");
///     Ok("Success".to_string())
/// }
/// ```
pub fn get_request_id(extensions: &axum::http::Extensions) -> Option<Uuid> {
    extensions.get::<Uuid>().copied()
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
    async fn test_request_id_generated() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(axum::middleware::from_fn(request_id_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should have X-Request-ID header
        assert!(response.headers().contains_key(REQUEST_ID_HEADER));

        // Should be valid UUID
        let request_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
        assert!(Uuid::parse_str(request_id.to_str().unwrap()).is_ok());
    }

    #[tokio::test]
    async fn test_request_id_preserved() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(axum::middleware::from_fn(request_id_middleware));

        let client_request_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, client_request_id.to_string())
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        // Should preserve client's request ID
        let response_request_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
        assert_eq!(response_request_id.to_str().unwrap(), client_request_id.to_string());
    }

    #[tokio::test]
    async fn test_invalid_request_id_replaced() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(axum::middleware::from_fn(request_id_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(REQUEST_ID_HEADER, "invalid-uuid")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap();

        // Should generate new valid UUID when client provides invalid one
        let request_id = response.headers().get(REQUEST_ID_HEADER).unwrap();
        assert!(Uuid::parse_str(request_id.to_str().unwrap()).is_ok());
    }
}

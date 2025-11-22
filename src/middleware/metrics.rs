// ============================================================================
// Prometheus Metrics Middleware - Production Implementation
// ============================================================================
//
// 📊 OBSERVABILITY: Full Prometheus metrics for production monitoring
//
// ## Metrics Collected:
//
// 1. **HTTP Request Duration**
//    - Histogram: atlas_http_request_duration_seconds
//    - Labels: method, path, status
//
// 2. **HTTP Request Total**
//    - Counter: atlas_http_requests_total
//    - Labels: method, path, status
//
// 3. **Active Connections**
//    - Gauge: atlas_http_connections_active
//
// 4. **Auth Failures**
//    - Counter: atlas_auth_failures_total
//    - Labels: reason
//
// ## Endpoints:
//
// - GET /metrics - Prometheus scrape endpoint
//
// ## Usage:
//
// ```rust
// // Add to router:
// .route("/metrics", get(metrics_handler))
//
// // Add middleware:
// .layer(middleware::from_fn(metrics_middleware))
// ```
//
// ## Prometheus Configuration:
//
// ```yaml
// scrape_configs:
//   - job_name: 'atlas-pharma'
//     static_configs:
//       - targets: ['localhost:8080']
//     metrics_path: '/metrics'
//     scrape_interval: 15s
// ```
//
// ============================================================================

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use lazy_static::lazy_static;
use prometheus::{
    Encoder, TextEncoder, HistogramVec, CounterVec, GaugeVec, Opts, Registry,
    register_histogram_vec, register_counter_vec, register_gauge_vec,
};
use std::time::Instant;

// ============================================================================
// PROMETHEUS METRICS REGISTRY
// ============================================================================

lazy_static! {
    /// HTTP request duration histogram
    /// Tracks response time distribution across all endpoints
    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "atlas_http_request_duration_seconds",
        "HTTP request latency in seconds",
        &["method", "path", "status"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]
    ).unwrap();

    /// HTTP request counter
    /// Counts total requests by method, path, and status
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "atlas_http_requests_total",
        "Total number of HTTP requests",
        &["method", "path", "status"]
    ).unwrap();

    /// Active HTTP connections gauge
    /// Tracks currently active connections
    pub static ref HTTP_CONNECTIONS_ACTIVE: GaugeVec = register_gauge_vec!(
        "atlas_http_connections_active",
        "Number of active HTTP connections",
        &[]
    ).unwrap();

    /// Authentication failures counter
    /// Tracks failed authentication attempts by reason
    pub static ref AUTH_FAILURES_TOTAL: CounterVec = register_counter_vec!(
        "atlas_auth_failures_total",
        "Total number of authentication failures",
        &["reason"]
    ).unwrap();

    /// Database pool connections gauge
    /// Tracks database connection pool state
    pub static ref DB_POOL_CONNECTIONS: GaugeVec = register_gauge_vec!(
        "atlas_db_pool_connections",
        "Database connection pool state",
        &["state"]
    ).unwrap();

    /// API quota usage gauge
    /// Tracks API quota usage percentage by user
    pub static ref API_QUOTA_USAGE_PERCENT: GaugeVec = register_gauge_vec!(
        "atlas_api_quota_usage_percent",
        "API quota usage percentage",
        &["user_id", "tier"]
    ).unwrap();
}

/// Simplify path for metrics (remove IDs)
///
/// Example: /api/users/123 -> /api/users/:id
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let mut normalized = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            continue;
        }

        // Check if segment looks like an ID (UUID or numeric)
        if segment.len() == 36 && segment.contains('-') {
            // Likely a UUID
            normalized.push(":id");
        } else if segment.parse::<i64>().is_ok() {
            // Numeric ID
            normalized.push(":id");
        } else {
            normalized.push(segment);
        }
    }

    format!("/{}", normalized.join("/"))
}

// ============================================================================
// METRICS MIDDLEWARE
// ============================================================================

/// Prometheus metrics middleware
///
/// Records HTTP request duration, counts, and active connections
///
pub async fn metrics_middleware(
    request: Request,
    next: Next,
) -> Response {
    // Increment active connections
    HTTP_CONNECTIONS_ACTIVE.with_label_values(&[]).inc();

    let start = Instant::now();
    let method = request.method().clone();
    let path = normalize_path(request.uri().path());

    // Process request
    let response = next.run(request).await;

    // Record metrics
    let duration = start.elapsed();
    let status = response.status();
    let status_str = status.as_u16().to_string();

    // Record duration histogram
    HTTP_REQUEST_DURATION
        .with_label_values(&[method.as_str(), &path, &status_str])
        .observe(duration.as_secs_f64());

    // Increment request counter
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method.as_str(), &path, &status_str])
        .inc();

    // Decrement active connections
    HTTP_CONNECTIONS_ACTIVE.with_label_values(&[]).dec();

    // Log metrics
    tracing::debug!(
        target: "metrics",
        method = %method,
        path = %path,
        status = %status.as_u16(),
        duration_ms = %duration.as_millis(),
        "HTTP request completed"
    );

    response
}

// ============================================================================
// METRICS ENDPOINT HANDLER
// ============================================================================

/// Metrics endpoint handler
///
/// Returns Prometheus-formatted metrics for scraping
///
pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(_) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            buffer
        ),
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
                format!("Failed to encode metrics: {}", e).into_bytes()
            )
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS FOR APPLICATION USE
// ============================================================================

/// Record authentication failure
///
/// Call this whenever authentication fails
///
/// # Example
/// ```
/// use atlas_pharma::middleware::metrics::record_auth_failure;
/// record_auth_failure("invalid_password");
/// ```
pub fn record_auth_failure(reason: &str) {
    AUTH_FAILURES_TOTAL.with_label_values(&[reason]).inc();
    tracing::warn!(target: "security", reason = %reason, "Authentication failure recorded");
}

/// Record database pool state
///
/// Call this periodically to track connection pool health
///
pub fn record_db_pool_state(idle: usize, active: usize) {
    DB_POOL_CONNECTIONS.with_label_values(&["idle"]).set(idle as f64);
    DB_POOL_CONNECTIONS.with_label_values(&["active"]).set(active as f64);
}

/// Record API quota usage
///
/// Call this after API quota checks
///
pub fn record_api_quota_usage(user_id: &str, tier: &str, usage_percent: f64) {
    API_QUOTA_USAGE_PERCENT
        .with_label_values(&[user_id, tier])
        .set(usage_percent);
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let response = metrics_handler().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/api/users/123"), "/api/users/:id");
        assert_eq!(
            normalize_path("/api/users/550e8400-e29b-41d4-a716-446655440000"),
            "/api/users/:id"
        );
        assert_eq!(normalize_path("/api/auth/login"), "/api/auth/login");
    }

    #[test]
    fn test_record_auth_failure() {
        record_auth_failure("invalid_password");
        let metric_families = prometheus::gather();
        assert!(!metric_families.is_empty());
    }
}

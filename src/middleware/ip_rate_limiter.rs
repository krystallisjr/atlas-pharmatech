/// Production-Grade IP-Based Rate Limiting
///
/// Thread-safe, in-memory rate limiter using token bucket algorithm
/// - Protects against brute force attacks
/// - Prevents DoS/DDoS
/// - Per-IP tracking with automatic cleanup
///
/// Industry standard: Redis-backed for distributed systems, in-memory for single instance

use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;

/// Rate limiter configuration
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window duration
    pub window: Duration,
}

impl RateLimitConfig {
    /// Strict limits for authentication endpoints (prevent brute force)
    pub fn auth() -> Self {
        Self {
            max_requests: 5,
            window: Duration::from_secs(60), // 5 requests per minute
        }
    }

    /// Relaxed limits for general API endpoints
    pub fn api() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60), // 100 requests per minute
        }
    }
}

/// Track requests per IP address
struct IpTracker {
    requests: Vec<Instant>,
    last_cleanup: Instant,
}

impl IpTracker {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            last_cleanup: Instant::now(),
        }
    }

    /// Remove expired requests and check if under limit
    fn check_limit(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();

        // Cleanup old requests
        self.requests.retain(|&req_time| {
            now.duration_since(req_time) < config.window
        });

        self.last_cleanup = now;

        // Check if under limit
        if self.requests.len() >= config.max_requests as usize {
            return false; // Rate limit exceeded
        }

        // Add new request
        self.requests.push(now);
        true
    }

    /// Get retry-after seconds
    fn retry_after(&self, config: &RateLimitConfig) -> u64 {
        if let Some(&oldest) = self.requests.first() {
            let elapsed = Instant::now().duration_since(oldest);
            let remaining = config.window.saturating_sub(elapsed);
            remaining.as_secs()
        } else {
            0
        }
    }
}

/// Production rate limiter with automatic cleanup
pub struct RateLimiter {
    trackers: Arc<DashMap<String, IpTracker>>,
    config: RateLimitConfig,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let window = config.window; // Clone before moving into Self
        let limiter = Self {
            trackers: Arc::new(DashMap::new()),
            config,
        };

        // Spawn cleanup task
        let trackers = limiter.trackers.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(300)).await; // Cleanup every 5 minutes
                trackers.retain(|_, tracker| {
                    // Remove trackers with no recent activity
                    Instant::now().duration_since(tracker.last_cleanup) < window * 2
                });
            }
        });

        limiter
    }

    /// Check if request is allowed
    pub fn check(&self, ip: &str) -> Result<(), u64> {
        let mut entry = self.trackers.entry(ip.to_string()).or_insert_with(IpTracker::new);

        if entry.check_limit(&self.config) {
            Ok(())
        } else {
            Err(entry.retry_after(&self.config))
        }
    }
}

/// Axum middleware for rate limiting
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    // Extract rate limiter from request extensions
    let limiter = request
        .extensions()
        .get::<Arc<RateLimiter>>()
        .cloned()
        .expect("RateLimiter not found in extensions");

    let ip = addr.ip().to_string();

    match limiter.check(&ip) {
        Ok(()) => Ok(next.run(request).await),
        Err(retry_after) => {
            tracing::warn!("Rate limit exceeded for IP: {}", ip);
            Err((
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", retry_after.to_string())],
                format!("Rate limit exceeded. Try again in {} seconds.", retry_after),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 3,
            window: Duration::from_secs(1),
        });

        assert!(limiter.check("127.0.0.1").is_ok());
        assert!(limiter.check("127.0.0.1").is_ok());
        assert!(limiter.check("127.0.0.1").is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 2,
            window: Duration::from_secs(10),
        });

        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_ok());
        assert!(limiter.check("192.168.1.1").is_err()); // Should be blocked
    }

    #[tokio::test]
    async fn test_different_ips_independent() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 1,
            window: Duration::from_secs(10),
        });

        assert!(limiter.check("10.0.0.1").is_ok());
        assert!(limiter.check("10.0.0.2").is_ok()); // Different IP, should work
    }

    #[tokio::test]
    async fn test_window_expiration() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_requests: 1,
            window: Duration::from_millis(100),
        });

        assert!(limiter.check("172.16.0.1").is_ok());
        assert!(limiter.check("172.16.0.1").is_err()); // Blocked

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(limiter.check("172.16.0.1").is_ok()); // Window expired, should work
    }
}

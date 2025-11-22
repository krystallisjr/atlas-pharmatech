use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
    extract::{State, Request},
    middleware::Next,
};
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, Any};
use axum::http::{HeaderValue, Method, header};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use atlas_pharma::config::AppConfig;
use atlas_pharma::middleware::ip_rate_limiter::{RateLimiter, RateLimitConfig};
use std::sync::Arc;
use atlas_pharma::handlers::{
    auth::{register, login, logout, get_profile, update_profile, delete_account, refresh_token},
    pharmaceutical::{
        create_pharmaceutical, get_pharmaceutical, search_pharmaceuticals,
        get_manufacturers, get_categories,
    },
    inventory::{
        add_inventory, get_inventory, get_user_inventory, update_inventory,
        delete_inventory, search_marketplace, get_expiry_alerts,
    },
    marketplace::{
        create_inquiry, get_inquiry, get_buyer_inquiries, get_seller_inquiries,
        update_inquiry_status, create_transaction, get_transaction,
        get_user_transactions, complete_transaction, cancel_transaction,
    },
    inquiry_messages::{
        create_message, get_inquiry_messages, get_message_count,
    },
    openfda::{
        search_catalog, get_by_ndc, get_stats, trigger_sync,
        get_manufacturers as get_openfda_manufacturers,
        get_sync_progress, get_active_sync, get_sync_logs as get_openfda_sync_logs,
        cancel_sync, check_refresh_status as openfda_check_refresh_status,
        cleanup_sync_logs as openfda_cleanup_sync_logs, health_check as openfda_health_check,
    },
    ema::{
        search_catalog as ema_search_catalog,
        get_by_eu_number,
        get_stats as ema_get_stats,
        trigger_sync as ema_trigger_sync,
        get_sync_logs,
        check_refresh_status,
        get_config_info as ema_get_config_info,
        cleanup_sync_logs,
        health_check as ema_health_check,
    },
    ai_import::{
        upload_and_analyze, list_sessions, get_session,
        start_import, get_session_rows, get_user_quota,
    },
    nl_query,
    inquiry_assistant,
    alerts,
};
use atlas_pharma::middleware::auth_middleware;

pub fn create_app(config: AppConfig) -> Router {
    // 🔒 PRODUCTION LOGGING CONFIGURATION
    // Default to INFO level (not DEBUG) to prevent verbose logging in production
    // Override with RUST_LOG environment variable for debugging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "atlas_pharma=info,tower_http=info,sqlx=warn".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 🔒 PRODUCTION RATE LIMITING
    let auth_rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig::auth()));
    let api_rate_limiter = Arc::new(RateLimiter::new(RateLimitConfig::api()));

    // 🔒 PRODUCTION TOKEN BLACKLIST (logout/revocation)
    let token_blacklist = Arc::new(atlas_pharma::services::TokenBlacklistService::new());

    // 📋 PRODUCTION AUDIT LOGGING (compliance: SOC 2, HIPAA, ISO 27001)
    let audit_service = Arc::new(atlas_pharma::services::ComprehensiveAuditService::new(config.database_pool.clone()));

    // 🔒 SECURITY: Strict CORS policy - only allow whitelisted origins
    // Validate CORS origins for security issues
    for origin in &config.cors_origins {
        // ⚠️  WARNING: Check for insecure origin patterns
        if origin.starts_with("http://") && !origin.contains("localhost") {
            tracing::warn!(
                "⚠️  SECURITY WARNING: Insecure HTTP origin in CORS: {} (use HTTPS in production!)",
                origin
            );
        }

        // ⚠️  WARNING: Check for IP-based origins (not recommended)
        if origin.contains("://") {
            let host_part = origin.split("://").nth(1).unwrap_or("");
            let host = host_part.split(':').next().unwrap_or("");
            if host.parse::<std::net::IpAddr>().is_ok() {
                tracing::warn!(
                    "⚠️  SECURITY WARNING: IP-based CORS origin: {} (use domain names in production!)",
                    origin
                );
            }
        }
    }

    let cors_origins: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|origin| {
            match origin.parse() {
                Ok(header_val) => Some(header_val),
                Err(e) => {
                    tracing::error!("❌ Invalid CORS origin '{}': {}", origin, e);
                    None
                }
            }
        })
        .collect();

    tracing::info!("✅ CORS configured with {} allowed origins", cors_origins.len());

    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_credentials(true)  // Required for httpOnly cookies
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::COOKIE,
        ]);

    let app = Router::new()
        .nest(
            "/api/auth",
            Router::new()
                // Public routes (no auth required)
                .route("/register", post(register))
                .route("/login", post(login))
                .route("/refresh", post(refresh_token))
                .layer(middleware::from_fn(atlas_pharma::middleware::ip_rate_limiter::rate_limit_middleware))  // 🔒 RATE LIMITING
                .layer(axum::Extension(auth_rate_limiter.clone()))  // Extension MUST be added before middleware
                // Protected routes (auth required)
                .merge(
                    Router::new()
                        .route("/logout", post(logout))
                        .route("/profile", get(get_profile))
                        .route("/profile", put(update_profile))
                        .route("/change-password", post(atlas_pharma::handlers::auth::change_password))  // 🔒 SECURITY: Password change with session invalidation
                        .route("/delete", delete(delete_account))
                        .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
                )
                // OAuth routes (public - redirect to provider)
                .merge(
                    Router::new()
                        .route("/oauth/providers", get(atlas_pharma::handlers::oauth::get_oauth_providers))
                        .route("/oauth/:provider", get(atlas_pharma::handlers::oauth::oauth_start))
                        .route("/oauth/:provider/callback", get(atlas_pharma::handlers::oauth::oauth_callback))
                )
                // OAuth account linking (auth required)
                .merge(
                    Router::new()
                        .route("/oauth/link/:provider", post(atlas_pharma::handlers::oauth::oauth_link_start))
                        .route("/oauth/unlink/:provider", post(atlas_pharma::handlers::oauth::oauth_unlink))
                        .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
                )
        )
        .nest(
            "/api/admin",
            Router::new()
                // Admin health check (public - for monitoring)
                .route("/health", get(atlas_pharma::handlers::admin::health_check))
                // Admin-only endpoints (require admin or superadmin role)
                .merge(
                    Router::new()
                        // User management
                        .route("/users", get(atlas_pharma::handlers::admin::list_users))
                        .route("/users/:id", get(atlas_pharma::handlers::admin::get_user))
                        .route("/users/:id/verify", post(atlas_pharma::handlers::admin::verify_user))
                        // Verification queue
                        .route("/verification-queue", get(atlas_pharma::handlers::admin::get_verification_queue))
                        // Statistics
                        .route("/stats", get(atlas_pharma::handlers::admin::get_admin_stats))
                        // Audit logs
                        .route("/audit-logs", get(atlas_pharma::handlers::admin::get_audit_logs))
                        // Security monitoring (read-only)
                        .route("/security/api-usage", get(atlas_pharma::handlers::admin_security::get_api_usage_analytics))
                        .route("/security/quotas", get(atlas_pharma::handlers::admin_security::get_user_quotas))
                        .route("/security/encryption", get(atlas_pharma::handlers::admin_security::get_encryption_status))
                        .route("/security/metrics", get(atlas_pharma::handlers::admin_security::get_metrics_summary))
                        .route("/security/rate-limits", get(atlas_pharma::handlers::admin_security::get_rate_limit_status))
                        .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
                        .layer(middleware::from_fn(atlas_pharma::middleware::admin_middleware))
                )
                // Superadmin-only endpoints (require superadmin role)
                .merge(
                    Router::new()
                        .route("/users/:id/role", put(atlas_pharma::handlers::admin::change_user_role))
                        .route("/users/:id", delete(atlas_pharma::handlers::admin::delete_user))
                        // Security management (write operations)
                        .route("/security/quotas/:user_id", put(atlas_pharma::handlers::admin_security::update_user_quota))
                        .route("/security/encryption/rotate", post(atlas_pharma::handlers::admin_security::rotate_encryption_key))
                        .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
                        .layer(middleware::from_fn(atlas_pharma::middleware::superadmin_middleware))
                )
        )
        .nest(
            "/api/mfa",
            Router::new()
                .route("/status", get(atlas_pharma::handlers::mfa::get_mfa_status))
                .route("/enroll/start", post(atlas_pharma::handlers::mfa::start_enrollment))
                .route("/enroll/complete", post(atlas_pharma::handlers::mfa::complete_enrollment))
                .route("/verify", post(atlas_pharma::handlers::mfa::verify_mfa))
                .route("/disable", post(atlas_pharma::handlers::mfa::disable_mfa))
                .route("/trusted-devices", get(atlas_pharma::handlers::mfa::get_trusted_devices))
                .route("/trusted-devices/:id", delete(atlas_pharma::handlers::mfa::revoke_trusted_device))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/pharmaceuticals",
            Router::new()
                .route("/", post(create_pharmaceutical))
                .route("/:id", get(get_pharmaceutical))
                .route("/search", get(search_pharmaceuticals))
                .route("/manufacturers", get(get_manufacturers))
                .route("/categories", get(get_categories))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/inventory",
            Router::new()
                .route("/", post(add_inventory))
                .route("/:id", get(get_inventory))
                .route("/my", get(get_user_inventory))
                .route("/:id", put(update_inventory))
                .route("/:id", delete(delete_inventory))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/marketplace",
            Router::new()
                .route("/search", get(search_marketplace))
                .route("/inquiries", post(create_inquiry))
                .route("/inquiries/:id", get(get_inquiry))
                .route("/inquiries/buyer", get(get_buyer_inquiries))
                .route("/inquiries/seller", get(get_seller_inquiries))
                .route("/inquiries/:id/status", put(update_inquiry_status))
                .route("/inquiries/:id/messages", get(get_inquiry_messages))
                .route("/inquiries/:id/messages", post(create_message))
                .route("/inquiries/:id/messages/count", get(get_message_count))
                .route("/transactions", post(create_transaction))
                .route("/transactions/:id", get(get_transaction))
                .route("/transactions/my", get(get_user_transactions))
                .route("/transactions/:id/complete", post(complete_transaction))
                .route("/transactions/:id/cancel", post(cancel_transaction))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/public",
            Router::new()
                .route("/inventory/search", get(search_marketplace))
                .route("/expiry-alerts", get(get_expiry_alerts))
        )
        .nest(
            "/api/openfda",
            Router::new()
                // Public endpoints
                .route("/search", get(search_catalog))
                .route("/ndc/:ndc", get(get_by_ndc))
                .route("/stats", get(get_stats))
                .route("/manufacturers", get(get_openfda_manufacturers))
                .route("/health", get(openfda_health_check))
                .route("/refresh-status", get(openfda_check_refresh_status))
                // Sync management (auth required)
                .route("/sync", post(trigger_sync))
                .route("/sync/active", get(get_active_sync))
                .route("/sync/logs", get(get_openfda_sync_logs))
                .route("/sync/:sync_id", get(get_sync_progress))
                .route("/sync/:sync_id/cancel", post(cancel_sync))
                .route("/cleanup", post(openfda_cleanup_sync_logs))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/ema",
            Router::new()
                .route("/search", get(ema_search_catalog))
                .route("/eu/:eu_number", get(get_by_eu_number))
                .route("/stats", get(ema_get_stats))
                .route("/sync", post(ema_trigger_sync))
                .route("/sync/logs", get(get_sync_logs))
                .route("/refresh-status", get(check_refresh_status))
                .route("/config", get(ema_get_config_info))
                .route("/cleanup", post(cleanup_sync_logs))
                .route("/health", get(ema_health_check))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/ai-import",
            Router::new()
                .route("/upload", post(upload_and_analyze))
                .route("/sessions", get(list_sessions))
                .route("/session/:id", get(get_session))
                .route("/session/:id/start-import", post(start_import))
                .route("/session/:id/rows", get(get_session_rows))
                .route("/quota", get(get_user_quota))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/nl-query",
            Router::new()
                .route("/execute", post(nl_query::execute_query))
                .route("/session/:id", get(nl_query::get_session))
                .route("/history", get(nl_query::get_history))
                .route("/favorites", post(nl_query::save_favorite))
                .route("/favorites", get(nl_query::get_favorites))
                .route("/quota", get(nl_query::get_quota))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/inquiry-assistant",
            Router::new()
                .route("/inquiries/:inquiry_id/suggestions", post(inquiry_assistant::generate_suggestion))
                .route("/suggestions/:suggestion_id", get(inquiry_assistant::get_suggestion))
                .route("/suggestions/:suggestion_id/accept", post(inquiry_assistant::accept_suggestion))
                .route("/inquiries/:inquiry_id/suggestions", get(inquiry_assistant::get_inquiry_suggestions))
                .route("/quota", get(inquiry_assistant::get_quota))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/alerts",
            Router::new()
                .route("/notifications", get(alerts::get_notifications))
                .route("/notifications/unread-count", get(alerts::get_unread_count))
                .route("/notifications/:id/read", put(alerts::mark_notification_read))
                .route("/notifications/mark-all-read", post(alerts::mark_all_read))
                .route("/notifications/:id", delete(alerts::dismiss_notification))
                .route("/preferences", get(alerts::get_preferences))
                .route("/preferences", put(alerts::update_preferences))
                .route("/watchlist", get(alerts::get_watchlists))
                .route("/watchlist", post(alerts::create_watchlist))
                .route("/watchlist/:id", get(alerts::get_watchlist))
                .route("/watchlist/:id", put(alerts::update_watchlist))
                .route("/watchlist/:id", delete(alerts::delete_watchlist))
                .route("/watchlist/:id/matches", get(alerts::get_watchlist_matches))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/regulatory",
            Router::new()
                .route("/documents/generate", post(atlas_pharma::handlers::regulatory_documents::generate_document))
                .route("/documents", get(atlas_pharma::handlers::regulatory_documents::list_documents))
                .route("/documents/:id", get(atlas_pharma::handlers::regulatory_documents::get_document))
                .route("/documents/:id/approve", post(atlas_pharma::handlers::regulatory_documents::approve_document))
                .route("/documents/:id/verify", get(atlas_pharma::handlers::regulatory_documents::verify_document))
                .route("/documents/:id/audit-trail", get(atlas_pharma::handlers::regulatory_documents::get_audit_trail))
                .route("/knowledge-base/stats", get(atlas_pharma::handlers::regulatory_documents::get_knowledge_base_stats))
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        .nest(
            "/api/erp",
            Router::new()
                // Connection management
                .route("/connections", post(atlas_pharma::handlers::erp_integration::create_connection))
                .route("/connections", get(atlas_pharma::handlers::erp_integration::list_connections))
                .route("/connections/:id", get(atlas_pharma::handlers::erp_integration::get_connection))
                .route("/connections/:id", delete(atlas_pharma::handlers::erp_integration::delete_connection))
                .route("/connections/:id/test", post(atlas_pharma::handlers::erp_integration::test_connection))
                // Sync operations
                .route("/connections/:id/sync", post(atlas_pharma::handlers::erp_integration::trigger_sync))
                .route("/connections/:id/sync-logs", get(atlas_pharma::handlers::erp_integration::get_sync_logs))
                // Mapping management
                .route("/connections/:id/mappings", get(atlas_pharma::handlers::erp_integration::get_mappings))
                .route("/mappings/:id", delete(atlas_pharma::handlers::erp_integration::delete_mapping))
                // AI-powered features
                .route("/connections/:id/auto-discover-mappings", post(atlas_pharma::handlers::erp_ai_integration::auto_discover_mappings))
                .route("/connections/:id/mapping-suggestions", get(atlas_pharma::handlers::erp_ai_integration::get_mapping_suggestions))
                .route("/connections/:id/mapping-suggestions/:suggestion_id/review", post(atlas_pharma::handlers::erp_ai_integration::review_mapping_suggestion))
                .route("/connections/:id/mapping-status", get(atlas_pharma::handlers::erp_ai_integration::get_mapping_status))
                .route("/sync-logs/:id/ai-analysis", get(atlas_pharma::handlers::erp_ai_integration::get_sync_analysis))
                .route("/connections/:id/resolve-conflicts", post(atlas_pharma::handlers::erp_ai_integration::suggest_conflict_resolution))
                // Webhooks (public endpoints - no auth middleware)
                .route("/webhooks/netsuite/:id", post(atlas_pharma::handlers::erp_integration::netsuite_webhook))
                .route("/webhooks/sap/:id", post(atlas_pharma::handlers::erp_integration::sap_webhook))
                .with_state(config.database_pool.clone())
                .layer(middleware::from_fn_with_state(config.clone(), auth_middleware))
        )
        // 📊 OBSERVABILITY: Prometheus metrics endpoint (public)
        .route("/metrics", get(atlas_pharma::middleware::metrics_handler))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(atlas_pharma::middleware::metrics_middleware))  // 📊 OBSERVABILITY: Prometheus metrics collection
                .layer(middleware::from_fn(atlas_pharma::middleware::content_type_validation_middleware))  // 🔒 SECURITY: Content-Type validation
                .layer(middleware::from_fn(atlas_pharma::middleware::request_id_middleware))  // 📊 OBSERVABILITY: Request ID tracking for distributed tracing
                .layer(middleware::from_fn(atlas_pharma::middleware::security_headers_middleware))  // 🔒 SECURITY: Production security headers (OWASP, PCI DSS, SOC 2)
                .layer(axum::Extension(audit_service.clone()))  // 📋 Audit logging for compliance
                .layer(axum::Extension(token_blacklist.clone()))  // 🔒 Token blacklist for logout/revocation
                .layer(axum::Extension(api_rate_limiter))  // 🔒 Rate limiter for DDoS protection
                .layer(middleware::from_fn(atlas_pharma::middleware::ip_rate_limiter::rate_limit_middleware))  // 🔒 Rate limiting middleware
                .layer(cors)
                .layer(axum::middleware::from_fn_with_state(
                    config.clone(),
                    |state: State<atlas_pharma::config::AppConfig>, req: Request<_>, next: Next| async move {
                        let auth_header = req
                            .headers()
                            .get(axum::http::header::AUTHORIZATION)
                            .and_then(|h| h.to_str().ok());

                        if let Some(auth_header) = auth_header {
                            if let Some(token) = atlas_pharma::middleware::JwtService::extract_token_from_header(auth_header) {
                                let jwt_service = atlas_pharma::middleware::JwtService::new(&state.jwt_secret);
                                if let Ok(claims) = jwt_service.validate_token(token) {
                                    let mut req = req;
                                    req.extensions_mut().insert(claims);
                                    return Ok::<axum::response::Response, atlas_pharma::middleware::error_handling::AppError>(next.run(req).await);
                                }
                            }
                        }

                        Ok::<axum::response::Response, atlas_pharma::middleware::error_handling::AppError>(next.run(req).await)
                    },
                ))
        )
        .with_state(config)
        .layer(axum::middleware::from_fn(
            |req: Request<_>, next: Next| async move {
                tracing::info!("{} {}", req.method(), req.uri());
                let response = next.run(req).await;
                tracing::info!("Response status: {}", response.status());
                response
            },
        ));

    app
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = atlas_pharma::config::AppConfig::from_env().await?;
    let tls_config = atlas_pharma::config::tls::TlsConfig::from_env()?;

    // Create app (this initializes the logger)
    let app = create_app(config.clone());

    // 🔒 SECURITY: Initialize API Quota Service
    tracing::info!("🔐 Initializing API Quota Service...");
    let quota_service = atlas_pharma::services::ApiQuotaService::new(config.database_pool.clone());

    // Initialize default quotas for existing users (if not already set)
    match quota_service.initialize_default_quotas().await {
        Ok(count) => tracing::info!("✅ API Quota Service initialized ({} users configured)", count),
        Err(e) => tracing::warn!("⚠️  Failed to initialize default quotas: {}", e),
    }

    // 🔐 SECURITY: Initialize Encryption Key Rotation Service
    tracing::info!("🔐 Initializing Encryption Key Rotation Service...");
    let key_rotation_service = atlas_pharma::services::EncryptionKeyRotationService::new(
        config.database_pool.clone(),
        config.encryption_key.clone()
    );

    // Initialize encryption keys if not already present
    match key_rotation_service.initialize().await {
        Ok(_) => tracing::info!("✅ Encryption Key Rotation Service initialized"),
        Err(e) => {
            tracing::error!("❌ Failed to initialize encryption keys: {}", e);
            tracing::warn!("⚠️  Application may not function correctly without encryption keys!");
        }
    }

    // Check if key rotation is recommended
    match key_rotation_service.get_rotation_recommendation().await {
        Ok(days_until) => {
            if days_until <= 7 {
                tracing::warn!("⚠️  Encryption key rotation recommended in {} days", days_until);
            } else if days_until <= 0 {
                tracing::error!("❌ Encryption key rotation OVERDUE by {} days", days_until.abs());
            } else {
                tracing::info!("✅ Next encryption key rotation in {} days", days_until);
            }
        }
        Err(e) => tracing::warn!("⚠️  Could not check key rotation status: {}", e),
    }

    // Start background alert scheduler
    let scheduler_pool = config.database_pool.clone();
    tokio::spawn(async move {
        use atlas_pharma::services::AlertSchedulerService;
        use std::time::Duration;

        let scheduler = AlertSchedulerService::new(scheduler_pool);
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); // Run every hour

        tracing::info!("🔔 Alert scheduler started - checking alerts every hour");

        loop {
            interval.tick().await;

            tracing::info!("🔄 Running scheduled alert checks...");

            match scheduler.run_scheduled_checks().await {
                Ok(stats) => {
                    tracing::info!(
                        "✅ Alert check completed: {} expiry, {} low stock, {} watchlist alerts generated",
                        stats.expiry_alerts_generated,
                        stats.low_stock_alerts_generated,
                        stats.watchlist_alerts_generated
                    );
                }
                Err(e) => {
                    tracing::error!("❌ Alert check failed: {}", e);
                }
            }
        }
    });

    // Start OpenFDA sync scheduler (weekly sync)
    let openfda_scheduler_pool = config.database_pool.clone();
    tokio::spawn(async move {
        use atlas_pharma::services::openfda_service::OpenFdaSyncScheduler;

        let scheduler = OpenFdaSyncScheduler::new(openfda_scheduler_pool);
        tracing::info!("📦 OpenFDA sync scheduler initialized");
        scheduler.run().await;
    });

    // Start server with TLS if enabled, otherwise use plain HTTP
    if tls_config.enabled {
        let rustls_config = tls_config.build_rustls_config().await?;
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], tls_config.port));

        tracing::info!("🔒 Starting Atlas Pharma server with TLS on https://{}", addr);

        axum_server::bind_rustls(addr, rustls_config)
            .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await?;
    } else {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));

        tracing::info!("⚠️  Starting Atlas Pharma server WITHOUT TLS on http://{}", addr);
        tracing::warn!("⚠️  TLS is DISABLED - This is NOT recommended for production!");
        tracing::info!("💡 To enable TLS, set TLS_ENABLED=true in .env and configure certificates");

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>()
        ).await?;
    }

    Ok(())
}
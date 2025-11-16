pub mod config;
pub mod models;
pub mod repositories;
pub mod services;
pub mod handlers;
pub mod middleware;
pub mod utils;

use std::net::SocketAddr;
use axum::{Router, response::{Response, IntoResponse}, extract::Request, body::Body, middleware::Next};
use anyhow::Result;
use config::AppConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::from_env().await?;
    
    let app = create_app(config);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    
    tracing::info!("Starting Atlas Pharma server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn create_app(config: AppConfig) -> Router {
    use axum::{
        middleware,
        routing::{get, post, put, delete},
        Router,
        response::{Response, IntoResponse},
    };
    use tower::ServiceBuilder;
    use tower_http::cors::{CorsLayer, Any};
    use crate::handlers::{
        auth::{register, login, get_profile, update_profile, delete_account, refresh_token},
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
    };
    use crate::middleware::auth_middleware;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest(
            "/api/auth",
            Router::new()
                .route("/register", post(register))
                .route("/login", post(login))
                .route("/refresh", post(refresh_token))
                .route("/profile", get(get_profile))
                .route("/profile", put(update_profile))
                .route("/delete", delete(delete_account))
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
                .route("/inquiries", post(create_inquiry))
                .route("/inquiries/:id", get(get_inquiry))
                .route("/inquiries/buyer", get(get_buyer_inquiries))
                .route("/inquiries/seller", get(get_seller_inquiries))
                .route("/inquiries/:id/status", put(update_inquiry_status))
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
        .layer(
            ServiceBuilder::new()
                .layer(cors)
                .layer(axum::middleware::from_fn_with_state(
                    config.clone(),
                    |state: axum::extract::State<crate::config::AppConfig>, req: Request<Body>, next: Next| async move {
                        let auth_header = req
                            .headers()
                            .get(axum::http::header::AUTHORIZATION)
                            .and_then(|h| h.to_str().ok());
                        
                        if let Some(auth_header) = auth_header {
                            if let Some(token) = crate::middleware::JwtService::extract_token_from_header(auth_header) {
                                let jwt_service = crate::middleware::JwtService::new(&state.jwt_secret);
                                if let Ok(claims) = jwt_service.validate_token(token) {
                                    let mut req = req;
                                    req.extensions_mut().insert(claims);
                                    return Ok::<axum::response::Response, crate::middleware::error_handling::AppError>(next.run(req).await);
                                }
                            }
                        }
                        
                        Ok::<axum::response::Response, crate::middleware::error_handling::AppError>(next.run(req).await)
                    },
                ))
        )
        .with_state(config)
        .layer(axum::middleware::from_fn(
            |req: Request<Body>, next: Next| async move {
                tracing::info!("{} {}", req.method(), req.uri());
                let response = next.run(req).await;
                tracing::info!("Response status: {}", response.status());
                response
            },
        ))
}
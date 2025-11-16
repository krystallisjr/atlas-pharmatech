use axum::{
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;
use validator::ValidationErrors;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationErrors),
    
    #[error("JSON error: {0}")]
    Json(#[from] JsonRejection),

    #[error("JSON parsing error: {0}")]
    JsonParsing(#[from] serde_json::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
    
    #[error("Password hashing error: {0}")]
    PasswordHash(#[from] bcrypt::BcryptError),
    
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict")]
    Conflict,

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("Encryption error: {0}")]
    Encryption(String),
}

impl From<crate::services::encryption_service::EncryptionError> for AppError {
    fn from(err: crate::services::encryption_service::EncryptionError) -> Self {
        AppError::Encryption(format!("{:?}", err))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::Validation(_) => (StatusCode::BAD_REQUEST, "Validation failed".to_string()),
            AppError::Json(_) => (StatusCode::BAD_REQUEST, "Invalid JSON".to_string()),
            AppError::JsonParsing(ref e) => (StatusCode::BAD_REQUEST, format!("JSON parsing error: {}", e)),
            AppError::Jwt(_) => (StatusCode::UNAUTHORIZED, "Invalid token".to_string()),
            AppError::PasswordHash(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Password processing error".to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Conflict => (StatusCode::CONFLICT, "Resource already exists".to_string()),
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::QuotaExceeded(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            AppError::TooManyRequests(msg) => (StatusCode::TOO_MANY_REQUESTS, msg),
            AppError::Internal(err) => {
                tracing::error!("Internal error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::Encryption(msg) => {
                tracing::error!("Encryption error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Encryption error".to_string())
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
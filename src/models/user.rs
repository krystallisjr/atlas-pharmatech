use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub company_name: String,
    pub contact_person: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub license_number: Option<String>,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,
    #[validate(length(min = 2, message = "Company name must be at least 2 characters"))]
    pub company_name: String,
    #[validate(length(min = 2, message = "Contact person name must be at least 2 characters"))]
    pub contact_person: String,
    #[validate(length(max = 20, message = "Phone number too long"))]
    pub phone: Option<String>,
    pub address: Option<String>,
    #[validate(length(max = 100, message = "License number too long"))]
    pub license_number: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password required"))]
    pub password: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub company_name: String,
    pub contact_person: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub license_number: Option<String>,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            company_name: user.company_name,
            contact_person: user.contact_person,
            phone: user.phone,
            address: user.address,
            license_number: user.license_number,
            is_verified: user.is_verified,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateUserRequest {
    #[validate(length(min = 2, message = "Company name must be at least 2 characters"))]
    pub company_name: Option<String>,
    #[validate(length(min = 2, message = "Contact person name must be at least 2 characters"))]
    pub contact_person: Option<String>,
    #[validate(length(max = 20, message = "Phone number too long"))]
    pub phone: Option<String>,
    pub address: Option<String>,
    #[validate(length(max = 100, message = "License number too long"))]
    pub license_number: Option<String>,
}
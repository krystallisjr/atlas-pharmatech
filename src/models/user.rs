use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

/// User role enum matching database user_role type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    User,
    Admin,
    Superadmin,
}

impl UserRole {
    /// Check if role has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Superadmin)
    }

    /// Check if role has superadmin privileges
    pub fn is_superadmin(&self) -> bool {
        matches!(self, UserRole::Superadmin)
    }

    /// Get role display name
    pub fn display_name(&self) -> &'static str {
        match self {
            UserRole::User => "User",
            UserRole::Admin => "Admin",
            UserRole::Superadmin => "Super Admin",
        }
    }
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

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
    pub role: UserRole,
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
    pub role: UserRole,
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
            role: user.role,
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
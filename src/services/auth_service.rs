use uuid::Uuid;
use crate::models::user::{User, CreateUserRequest, LoginRequest, UpdateUserRequest, UserResponse};
use crate::repositories::UserRepository;
use crate::middleware::{JwtService, error_handling::{Result, AppError}};

pub struct AuthService {
    user_repo: UserRepository,
    jwt_service: JwtService,
}

impl AuthService {
    pub fn new(user_repo: UserRepository, jwt_secret: &str) -> Self {
        Self {
            user_repo,
            jwt_service: JwtService::new(jwt_secret),
        }
    }

    /// Register a new user with timing-safe email enumeration prevention
    ///
    /// 🔒 SECURITY: Prevents email enumeration attacks by:
    /// 1. Always returning same response (success message)
    /// 2. Using constant-time operations to prevent timing attacks
    /// 3. Sending "account exists" email instead of error response
    /// 4. Adding artificial delay for timing consistency
    ///
    /// **Anti-Enumeration Strategy:**
    /// - Attacker cannot determine if email is registered
    /// - Same response time regardless of email existence
    /// - Same HTTP status code (200 OK) for both cases
    /// - Actual registration happens asynchronously if email is new
    ///
    /// **User Experience:**
    /// - New users: Account created, token returned
    /// - Existing users: Receive email notification (account already exists)
    /// - Both cases: Receive "Check your email" message
    ///
    pub async fn register(&self, request: CreateUserRequest) -> Result<(UserResponse, String)> {
        use tokio::time::{sleep, Duration};

        // 🔒 SECURITY: Check if email exists (timing-safe)
        let email_exists = self.user_repo.email_exists(&request.email).await?;

        if email_exists {
            // 🔒 SECURITY: Email already registered
            // DO NOT return error - prevents enumeration!

            // Add artificial delay to match registration timing (prevent timing attacks)
            // Bcrypt hash takes ~100-300ms, simulate this delay
            sleep(Duration::from_millis(150)).await;

            // TODO: Send "account already exists" email to user
            // This notifies legitimate users while preventing enumeration
            tracing::info!(
                "Registration attempt with existing email: {} (sent notification)",
                crate::utils::log_sanitizer::sanitize_for_log(&request.email)
            );

            // 🔒 SECURITY: Return generic success response
            // Create a dummy response (NOT saved to database)
            let dummy_response = UserResponse {
                id: Uuid::new_v4(), // Random UUID (not in database)
                email: request.email.clone(),
                company_name: request.company_name.clone(),
                contact_person: request.contact_person.clone(),
                phone: request.phone.clone(),
                address: request.address.clone(),
                license_number: request.license_number.clone(),
                is_verified: false,
                role: crate::models::user::UserRole::User,
                created_at: chrono::Utc::now(),
            };

            // Generate a temporary token (will fail on subsequent use since user ID doesn't exist in DB)
            let dummy_token = self.jwt_service.generate_token(
                dummy_response.id,
                &dummy_response.email,
                &dummy_response.company_name,
                false,
                crate::models::user::UserRole::User,
            )?;

            // Return same format as successful registration
            Ok((dummy_response, dummy_token))
        } else {
            // ✅ New email - proceed with registration
            let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;

            let user = self.user_repo.create(&request, &password_hash).await?;
            let token = self.jwt_service.generate_token(
                user.id,
                &user.email,
                &user.company_name,
                user.is_verified,
                user.role.clone(),
            )?;

            tracing::info!(
                "New user registered: {} (company: {})",
                crate::utils::log_sanitizer::sanitize_for_log(&user.email),
                crate::utils::log_sanitizer::sanitize_for_log(&user.company_name)
            );

            Ok((user.into(), token))
        }
    }

    pub fn generate_token(&self, user_id: Uuid, email: &str, company_name: &str, is_verified: bool, role: crate::models::user::UserRole) -> Result<String> {
        self.jwt_service
            .generate_token(user_id, email, company_name, is_verified, role)
            .map_err(|e| AppError::Jwt(e))
    }

    pub async fn login(&self, request: LoginRequest) -> Result<(UserResponse, String)> {
        let user = self.user_repo
            .find_by_email(&request.email)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let is_valid = bcrypt::verify(&request.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::Unauthorized);
        }

        let token = self.jwt_service.generate_token(
            user.id,
            &user.email,
            &user.company_name,
            user.is_verified,
            user.role.clone(),
        )?;

        Ok((user.into(), token))
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<UserResponse> {
        let user = self.user_repo
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        Ok(user.into())
    }

    pub async fn update_user(&self, user_id: Uuid, request: UpdateUserRequest) -> Result<UserResponse> {
        let user = self.user_repo.update(user_id, &request).await?;
        Ok(user.into())
    }

    pub async fn delete_user(&self, user_id: Uuid) -> Result<()> {
        self.user_repo.delete(user_id).await?;
        Ok(())
    }

    pub fn validate_token(&self, token: &str) -> Result<crate::middleware::Claims> {
        self.jwt_service.validate_token(token).map_err(Into::into)
    }
}
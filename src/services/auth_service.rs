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

    pub async fn register(&self, request: CreateUserRequest) -> Result<(UserResponse, String)> {
        if self.user_repo.email_exists(&request.email).await? {
            return Err(AppError::Conflict);
        }

        let password_hash = bcrypt::hash(&request.password, bcrypt::DEFAULT_COST)?;
        
        let user = self.user_repo.create(&request, &password_hash).await?;
        let token = self.jwt_service.generate_token(
            user.id,
            &user.email,
            &user.company_name,
            user.is_verified,
        )?;

        Ok((user.into(), token))
    }

    pub fn generate_token(&self, user_id: Uuid, email: &str, company_name: &str, is_verified: bool) -> Result<String> {
        self.jwt_service
            .generate_token(user_id, email, company_name, is_verified)
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
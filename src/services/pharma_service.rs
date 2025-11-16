use uuid::Uuid;
use crate::models::pharmaceutical::{Pharmaceutical, CreatePharmaceuticalRequest, SearchPharmaceuticalRequest, PharmaceuticalResponse};
use crate::repositories::PharmaceuticalRepository;
use crate::middleware::error_handling::{Result, AppError};

pub struct PharmaService {
    pharma_repo: PharmaceuticalRepository,
}

impl PharmaService {
    pub fn new(pharma_repo: PharmaceuticalRepository) -> Self {
        Self { pharma_repo }
    }

    pub async fn create_pharmaceutical(&self, request: CreatePharmaceuticalRequest) -> Result<PharmaceuticalResponse> {
        // If NDC provided, use find-or-create pattern to avoid duplicates
        if let Some(ref ndc_code) = request.ndc_code {
            // First, check if it already exists
            if let Some(existing) = self.pharma_repo.find_by_ndc(ndc_code).await? {
                return Ok(existing.into());
            }
        }

        // Try to create, but handle potential race condition with constraint violation
        match self.pharma_repo.create(&request).await {
            Ok(pharma) => Ok(pharma.into()),
            Err(e) => {
                // Check if it's a database error with unique constraint violation
                if let AppError::Database(ref db_err) = e {
                    if let Some(db_error) = db_err.as_database_error() {
                        if db_error.code().as_deref() == Some("23505") {
                            // Constraint violation - pharmaceutical was created by another request
                            // Try to find it again
                            if let Some(ref ndc_code) = request.ndc_code {
                                if let Some(existing) = self.pharma_repo.find_by_ndc(ndc_code).await? {
                                    return Ok(existing.into());
                                }
                            }
                            // If we can't find it, return conflict error
                            return Err(AppError::Conflict);
                        }
                    }
                }
                // Not a constraint violation, propagate the error
                Err(e)
            }
        }
    }

    pub async fn get_pharmaceutical(&self, id: Uuid) -> Result<PharmaceuticalResponse> {
        let pharma = self.pharma_repo
            .find_by_id(id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        Ok(pharma.into())
    }

    pub async fn search_pharmaceuticals(&self, request: SearchPharmaceuticalRequest) -> Result<Vec<PharmaceuticalResponse>> {
        let pharmaceuticals = self.pharma_repo.search(&request).await?;
        Ok(pharmaceuticals.into_iter().map(Into::into).collect())
    }

    pub async fn get_manufacturers(&self) -> Result<Vec<String>> {
        self.pharma_repo.get_manufacturers().await
    }

    pub async fn get_categories(&self) -> Result<Vec<String>> {
        self.pharma_repo.get_categories().await
    }

    pub async fn find_or_create_by_ndc(&self, ndc_code: &str, request: CreatePharmaceuticalRequest) -> Result<PharmaceuticalResponse> {
        if let Some(pharma) = self.pharma_repo.find_by_ndc(ndc_code).await? {
            return Ok(pharma.into());
        }

        let pharma = self.pharma_repo.create(&request).await?;
        Ok(pharma.into())
    }

    pub async fn validate_pharmaceutical_exists(&self, id: Uuid) -> Result<bool> {
        let pharma = self.pharma_repo.find_by_id(id).await?;
        Ok(pharma.is_some())
    }
}
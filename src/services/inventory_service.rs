use uuid::Uuid;
use crate::models::{
    inventory::{Inventory, CreateInventoryRequest, UpdateInventoryRequest, SearchInventoryRequest, InventoryResponse, ExpiryAlert},
    user::UserResponse,
    pharmaceutical::PharmaceuticalResponse,
};
use crate::repositories::{InventoryRepository, PharmaceuticalRepository};
use crate::middleware::error_handling::{Result, AppError};
use chrono::NaiveDate;

pub struct InventoryService {
    inventory_repo: InventoryRepository,
    pharma_repo: PharmaceuticalRepository,
}

impl InventoryService {
    pub fn new(inventory_repo: InventoryRepository, pharma_repo: PharmaceuticalRepository) -> Self {
        Self { 
            inventory_repo,
            pharma_repo,
        }
    }

    pub async fn add_inventory(&self, request: CreateInventoryRequest, user_id: Uuid) -> Result<InventoryResponse> {
        if !self.pharma_repo.find_by_id(request.pharmaceutical_id).await?.is_some() {
            return Err(AppError::InvalidInput("Pharmaceutical not found".to_string()));
        }

        if self.inventory_repo.batch_exists(user_id, request.pharmaceutical_id, &request.batch_number).await? {
            return Err(AppError::Conflict);
        }

        let inventory = self.inventory_repo.create(&request, user_id).await?;
        self.to_response(inventory).await
    }

    pub async fn get_inventory(&self, inventory_id: Uuid, user_id: Uuid) -> Result<InventoryResponse> {
        let inventory = self.inventory_repo
            .find_by_id(inventory_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        if inventory.user_id != user_id {
            return Err(AppError::Forbidden("Access denied".to_string()));
        }

        self.to_response(inventory).await
    }

    pub async fn get_user_inventory(&self, user_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<InventoryResponse>> {
        let inventories = self.inventory_repo.find_by_user(user_id, limit, offset).await?;
        
        let mut responses = Vec::new();
        for inventory in inventories {
            responses.push(self.to_response(inventory).await?);
        }

        Ok(responses)
    }

    pub async fn search_marketplace(&self, request: SearchInventoryRequest) -> Result<Vec<InventoryResponse>> {
        let results = self.inventory_repo.search_with_details(&request).await?;
        
        let mut responses = Vec::new();
        for result in results {
            responses.push(self.to_response_with_details(result).await?);
        }

        Ok(responses)
    }

    pub async fn update_inventory(&self, inventory_id: Uuid, user_id: Uuid, request: UpdateInventoryRequest) -> Result<InventoryResponse> {
        let inventory = self.inventory_repo.update(inventory_id, user_id, &request).await?;
        self.to_response(inventory).await
    }

    pub async fn delete_inventory(&self, inventory_id: Uuid, user_id: Uuid) -> Result<()> {
        self.inventory_repo.delete(inventory_id, user_id).await?;
        Ok(())
    }

    pub async fn get_expiry_alerts(&self, days_threshold: i64) -> Result<Vec<ExpiryAlert>> {
        let results = self.inventory_repo.get_expiry_alerts(days_threshold).await?;
        
        let mut alerts = Vec::new();
        for result in results {
            alerts.push(ExpiryAlert {
                inventory_id: result.inventory.id,
                pharmaceutical_name: format!("{} ({})", result.pharmaceutical.brand_name, result.pharmaceutical.generic_name),
                batch_number: result.inventory.batch_number,
                quantity: result.inventory.quantity,
                expiry_date: result.inventory.expiry_date,
                days_to_expiry: result.inventory.expiry_date.signed_duration_since(chrono::Utc::now().date_naive()).num_days(),
                seller: result.user.company_name,
            });
        }

        Ok(alerts)
    }

    async fn to_response(&self, inventory: Inventory) -> Result<InventoryResponse> {
        let pharmaceutical = self.pharma_repo
            .find_by_id(inventory.pharmaceutical_id)
            .await?
            .ok_or(AppError::InvalidInput("Pharmaceutical not found".to_string()))?;

        let user_response = UserResponse {
            id: inventory.user_id,
            email: String::new(),
            company_name: String::new(),
            contact_person: String::new(),
            phone: None,
            address: None,
            license_number: None,
            is_verified: false,
            created_at: chrono::Utc::now(),
        };

        let days_to_expiry = inventory.expiry_date.signed_duration_since(chrono::Utc::now().date_naive()).num_days();

        Ok(InventoryResponse {
            id: inventory.id,
            pharmaceutical: pharmaceutical.into(),
            batch_number: inventory.batch_number,
            quantity: inventory.quantity,
            expiry_date: inventory.expiry_date,
            days_to_expiry,
            unit_price: inventory.unit_price,
            storage_location: inventory.storage_location,
            status: inventory.status,
            seller: user_response,
            created_at: inventory.created_at,
            updated_at: inventory.updated_at,
        })
    }

    async fn to_response_with_details(&self, result: crate::models::inventory::InventoryWithDetails) -> Result<InventoryResponse> {
        let days_to_expiry = result.inventory.expiry_date.signed_duration_since(chrono::Utc::now().date_naive()).num_days();

        Ok(InventoryResponse {
            id: result.inventory.id,
            pharmaceutical: result.pharmaceutical,
            batch_number: result.inventory.batch_number,
            quantity: result.inventory.quantity,
            expiry_date: result.inventory.expiry_date,
            days_to_expiry,
            unit_price: result.inventory.unit_price,
            storage_location: result.inventory.storage_location,
            status: result.inventory.status,
            seller: result.user,
            created_at: result.inventory.created_at,
            updated_at: result.inventory.updated_at,
        })
    }

    pub async fn reserve_inventory(&self, inventory_id: Uuid, quantity: i32) -> Result<()> {
        let inventory = self.inventory_repo
            .find_by_id(inventory_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        if inventory.quantity < quantity {
            return Err(AppError::InvalidInput("Insufficient inventory".to_string()));
        }

        let new_quantity = inventory.quantity - quantity;
        let update_request = UpdateInventoryRequest {
            quantity: Some(new_quantity),
            expiry_date: None,
            unit_price: None,
            storage_location: None,
            status: Some("reserved".to_string()),
        };

        self.inventory_repo.update(inventory_id, inventory.user_id, &update_request).await?;
        Ok(())
    }

    pub async fn release_inventory(&self, inventory_id: Uuid, quantity: i32) -> Result<()> {
        let inventory = self.inventory_repo
            .find_by_id(inventory_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        let new_quantity = inventory.quantity + quantity;
        let update_request = UpdateInventoryRequest {
            quantity: Some(new_quantity),
            expiry_date: None,
            unit_price: None,
            storage_location: None,
            status: Some("available".to_string()),
        };

        self.inventory_repo.update(inventory_id, inventory.user_id, &update_request).await?;
        Ok(())
    }
}
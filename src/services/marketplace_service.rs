use uuid::Uuid;
use crate::models::{
    marketplace::{Inquiry, CreateInquiryRequest, UpdateInquiryRequest, CreateTransactionRequest, TransactionResponse, InquiryResponse},
    inventory::InventoryResponse,
};
use crate::repositories::{MarketplaceRepository, InventoryRepository, UserRepository, PharmaceuticalRepository};
use crate::services::InventoryService;
use crate::middleware::error_handling::{Result, AppError};

pub struct MarketplaceService {
    marketplace_repo: MarketplaceRepository,
    inventory_repo: InventoryRepository,
    user_repo: UserRepository,
    pharma_repo: PharmaceuticalRepository,
    inventory_service: InventoryService,
}

impl MarketplaceService {
    pub fn new(
        marketplace_repo: MarketplaceRepository,
        inventory_repo: InventoryRepository,
        user_repo: UserRepository,
        pharma_repo: PharmaceuticalRepository,
        inventory_service: InventoryService,
    ) -> Self {
        Self {
            marketplace_repo,
            inventory_repo,
            user_repo,
            pharma_repo,
            inventory_service,
        }
    }

    pub async fn create_inquiry(&self, request: CreateInquiryRequest, buyer_id: Uuid) -> Result<InquiryResponse> {
        let inventory = self.inventory_repo
            .find_by_id(request.inventory_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        if inventory.user_id == buyer_id {
            return Err(AppError::InvalidInput("Cannot inquire about your own inventory".to_string()));
        }

        if self.marketplace_repo.inquiry_exists_for_buyer(request.inventory_id, buyer_id).await? {
            return Err(AppError::Conflict);
        }

        if request.quantity_requested > inventory.quantity {
            return Err(AppError::InvalidInput("Requested quantity exceeds available inventory".to_string()));
        }

        let inquiry = self.marketplace_repo.create_inquiry(&request, buyer_id).await?;
        Ok(inquiry.into())
    }

    pub async fn get_inquiry(&self, inquiry_id: Uuid, user_id: Uuid) -> Result<InquiryResponse> {
        if !self.marketplace_repo.can_access_inquiry(inquiry_id, user_id).await? {
            return Err(AppError::Forbidden("Access denied".to_string()));
        }

        let inquiry = self.marketplace_repo
            .find_inquiry_by_id(inquiry_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        self.enrich_inquiry(inquiry).await
    }

    // Helper method to enrich inquiry with nested data
    async fn enrich_inquiry(&self, inquiry: Inquiry) -> Result<InquiryResponse> {
        // Fetch inventory
        let inventory_opt = self.inventory_repo.find_by_id(inquiry.inventory_id).await?;

        // Build InventoryResponse with nested data if inventory exists
        let inventory_response = if let Some(inv) = inventory_opt {
            // Fetch pharmaceutical details
            let pharma = self.pharma_repo.find_by_id(inv.pharmaceutical_id).await?
                .map(Into::into);

            // Fetch seller (inventory owner)
            let seller = self.user_repo.find_by_id(inv.user_id).await?
                .map(Into::into);

            if let (Some(pharma), Some(seller)) = (pharma, seller) {
                let days_to_expiry = inv.expiry_date.signed_duration_since(chrono::Utc::now().date_naive()).num_days();
                Some(crate::models::inventory::InventoryResponse {
                    id: inv.id,
                    pharmaceutical: pharma,
                    batch_number: inv.batch_number,
                    quantity: inv.quantity,
                    expiry_date: inv.expiry_date,
                    days_to_expiry,
                    unit_price: inv.unit_price,
                    storage_location: inv.storage_location,
                    status: inv.status,
                    seller,
                    created_at: inv.created_at,
                    updated_at: inv.updated_at,
                })
            } else {
                None
            }
        } else {
            None
        };

        // Fetch buyer user
        let buyer = self.user_repo
            .find_by_id(inquiry.buyer_id)
            .await?
            .map(Into::into);

        // Get seller from inventory response
        let seller = inventory_response.as_ref().map(|inv| inv.seller.clone());

        Ok(InquiryResponse {
            id: inquiry.id,
            inventory_id: inquiry.inventory_id,
            buyer_id: inquiry.buyer_id,
            quantity_requested: inquiry.quantity_requested,
            message: inquiry.message,
            status: inquiry.status,
            created_at: inquiry.created_at,
            updated_at: inquiry.updated_at,
            inventory: inventory_response,
            buyer,
            seller,
        })
    }

    pub async fn get_buyer_inquiries(&self, buyer_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<InquiryResponse>> {
        let inquiries = self.marketplace_repo.get_inquiries_for_buyer(buyer_id, limit, offset).await?;

        let mut enriched = Vec::new();
        for inquiry in inquiries {
            enriched.push(self.enrich_inquiry(inquiry).await?);
        }
        Ok(enriched)
    }

    pub async fn get_seller_inquiries(&self, seller_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<InquiryResponse>> {
        let inquiries = self.marketplace_repo.get_inquiries_for_seller(seller_id, limit, offset).await?;

        let mut enriched = Vec::new();
        for inquiry in inquiries {
            enriched.push(self.enrich_inquiry(inquiry).await?);
        }
        Ok(enriched)
    }

    pub async fn update_inquiry_status(&self, inquiry_id: Uuid, user_id: Uuid, request: UpdateInquiryRequest) -> Result<InquiryResponse> {
        let inquiry = self.marketplace_repo
            .find_inquiry_by_id(inquiry_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        let inventory = self.inventory_repo
            .find_by_id(inquiry.inventory_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        if inventory.user_id != user_id {
            return Err(AppError::Forbidden("Access denied".to_string()));
        }

        if let Some(ref status) = request.status {
            match status.as_str() {
                "accepted" => {
                    if inquiry.quantity_requested > inventory.quantity {
                        return Err(AppError::InvalidInput("Insufficient inventory".to_string()));
                    }
                    self.inventory_service.reserve_inventory(inventory.id, inquiry.quantity_requested).await?;
                }
                "rejected" => {
                }
                _ => return Err(AppError::InvalidInput("Invalid status".to_string())),
            }
        }

        let updated_inquiry = self.marketplace_repo.update_inquiry(inquiry_id, &request).await?;
        Ok(updated_inquiry.into())
    }

    pub async fn create_transaction(&self, request: CreateTransactionRequest, seller_id: Uuid, buyer_id: Uuid) -> Result<TransactionResponse> {
        let inquiry = self.marketplace_repo
            .find_inquiry_by_id(request.inquiry_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        let inventory = self.inventory_repo
            .find_by_id(inquiry.inventory_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        if inventory.user_id != seller_id || inquiry.buyer_id != buyer_id {
            return Err(AppError::Forbidden("Access denied".to_string()));
        }

        if request.quantity > inquiry.quantity_requested {
            return Err(AppError::InvalidInput("Transaction quantity exceeds inquiry amount".to_string()));
        }

        let transaction = self.marketplace_repo.create_transaction(&request, seller_id, buyer_id).await?;
        Ok(transaction.into())
    }

    pub async fn get_transaction(&self, transaction_id: Uuid, user_id: Uuid) -> Result<TransactionResponse> {
        if !self.marketplace_repo.can_access_transaction(transaction_id, user_id).await? {
            return Err(AppError::Forbidden("Access denied".to_string()));
        }

        let transaction = self.marketplace_repo
            .find_transaction_by_id(transaction_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        Ok(transaction.into())
    }

    pub async fn get_user_transactions(&self, user_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<TransactionResponse>> {
        let transactions = self.marketplace_repo.get_transactions_for_user(user_id, limit, offset).await?;
        Ok(transactions.into_iter().map(Into::into).collect())
    }

    pub async fn complete_transaction(&self, transaction_id: Uuid, user_id: Uuid) -> Result<TransactionResponse> {
        let transaction = self.marketplace_repo
            .find_transaction_by_id(transaction_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        if transaction.seller_id != user_id {
            return Err(AppError::Forbidden("Access denied".to_string()));
        }

        if transaction.status != "pending" {
            return Err(AppError::InvalidInput("Transaction is not pending".to_string()));
        }

        let updated_transaction = self.marketplace_repo.update_transaction_status(transaction_id, "completed").await?;
        Ok(updated_transaction.into())
    }

    pub async fn cancel_transaction(&self, transaction_id: Uuid, user_id: Uuid) -> Result<TransactionResponse> {
        let transaction = self.marketplace_repo
            .find_transaction_by_id(transaction_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        if transaction.buyer_id != user_id && transaction.seller_id != user_id {
            return Err(AppError::Forbidden("Access denied".to_string()));
        }

        if transaction.status == "completed" {
            return Err(AppError::InvalidInput("Cannot cancel completed transaction".to_string()));
        }

        let updated_transaction = self.marketplace_repo.update_transaction_status(transaction_id, "cancelled").await?;
        
        let inquiry = self.marketplace_repo
            .find_inquiry_by_id(transaction.inquiry_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        let inventory = self.inventory_repo
            .find_by_id(inquiry.inventory_id)
            .await?
            .ok_or(AppError::NotFound("Resource not found".to_string()))?;

        self.inventory_service.release_inventory(inventory.id, transaction.quantity).await?;

        Ok(updated_transaction.into())
    }
}
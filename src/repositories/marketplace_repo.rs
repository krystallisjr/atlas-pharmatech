use sqlx::{PgPool, query, Row};
use uuid::Uuid;
use crate::models::marketplace::{Inquiry, CreateInquiryRequest, UpdateInquiryRequest, Transaction, CreateTransactionRequest};
use crate::middleware::error_handling::{Result, AppError};

pub struct MarketplaceRepository {
    pool: PgPool,
}

impl MarketplaceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_inquiry(&self, request: &CreateInquiryRequest, buyer_id: Uuid) -> Result<Inquiry> {
        let row = query(
            r#"
            INSERT INTO inquiries (inventory_id, buyer_id, quantity_requested, message, status)
            VALUES ($1, $2, $3, $4, 'pending')
            RETURNING id, inventory_id, buyer_id, quantity_requested, message, status, created_at, updated_at
            "#
        )
        .bind(&request.inventory_id)
        .bind(buyer_id)
        .bind(request.quantity_requested)
        .bind(&request.message)
        .fetch_one(&self.pool)
        .await?;

        let inquiry_id: Uuid = row.try_get("id")?;

        // Also insert the initial message into inquiry_messages table
        // so it shows up in the message thread for both buyer and seller
        if let Some(ref message) = request.message {
            if !message.trim().is_empty() {
                query(
                    r#"
                    INSERT INTO inquiry_messages (inquiry_id, sender_id, message)
                    VALUES ($1, $2, $3)
                    "#
                )
                .bind(inquiry_id)
                .bind(buyer_id)
                .bind(message.trim())
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(Inquiry {
            id: inquiry_id,
            inventory_id: row.try_get("inventory_id")?,
            buyer_id: row.try_get("buyer_id")?,
            quantity_requested: row.try_get("quantity_requested")?,
            message: row.try_get("message")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn find_inquiry_by_id(&self, id: Uuid) -> Result<Option<Inquiry>> {
        let row = query(
            "SELECT id, inventory_id, buyer_id, quantity_requested, message, status, created_at, updated_at FROM inquiries WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Inquiry {
                id: row.try_get("id")?,
                inventory_id: row.try_get("inventory_id")?,
                buyer_id: row.try_get("buyer_id")?,
                quantity_requested: row.try_get("quantity_requested")?,
                message: row.try_get("message")?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_inquiries_for_buyer(&self, buyer_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Inquiry>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let rows = query(
            "SELECT id, inventory_id, buyer_id, quantity_requested, message, status, created_at, updated_at 
             FROM inquiries WHERE buyer_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(buyer_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut inquiries = Vec::new();
        for row in rows {
            inquiries.push(Inquiry {
                id: row.try_get("id")?,
                inventory_id: row.try_get("inventory_id")?,
                buyer_id: row.try_get("buyer_id")?,
                quantity_requested: row.try_get("quantity_requested")?,
                message: row.try_get("message")?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(inquiries)
    }

    pub async fn get_inquiries_for_seller(&self, seller_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Inquiry>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let rows = query(
            r#"
            SELECT i.id, i.inventory_id, i.buyer_id, i.quantity_requested, i.message, i.status, i.created_at, i.updated_at
            FROM inquiries i
            JOIN inventory inv ON i.inventory_id = inv.id
            WHERE inv.user_id = $1
            ORDER BY i.created_at DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(seller_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut inquiries = Vec::new();
        for row in rows {
            inquiries.push(Inquiry {
                id: row.try_get("id")?,
                inventory_id: row.try_get("inventory_id")?,
                buyer_id: row.try_get("buyer_id")?,
                quantity_requested: row.try_get("quantity_requested")?,
                message: row.try_get("message")?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(inquiries)
    }

    pub async fn update_inquiry(&self, inquiry_id: Uuid, request: &UpdateInquiryRequest) -> Result<Inquiry> {
        let mut query_str = "UPDATE inquiries SET updated_at = CURRENT_TIMESTAMP".to_string();
        let mut param_count = 1;

        if request.status.is_some() {
            query_str.push_str(&format!(", status = ${}", param_count));
            param_count += 1;
        }

        query_str.push_str(&format!(" WHERE id = ${} RETURNING id, inventory_id, buyer_id, quantity_requested, message, status, created_at, updated_at", param_count));

        let mut query_builder = query(&query_str);

        if let Some(ref status) = request.status {
            query_builder = query_builder.bind(status);
        }

        let row = query_builder
            .bind(inquiry_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(Inquiry {
            id: row.try_get("id")?,
            inventory_id: row.try_get("inventory_id")?,
            buyer_id: row.try_get("buyer_id")?,
            quantity_requested: row.try_get("quantity_requested")?,
            message: row.try_get("message")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn create_transaction(&self, request: &CreateTransactionRequest, seller_id: Uuid, buyer_id: Uuid) -> Result<Transaction> {
        let total_price = rust_decimal::Decimal::from(request.quantity) * request.unit_price;

        let row = query(
            r#"
            INSERT INTO transactions (inquiry_id, seller_id, buyer_id, quantity, unit_price, total_price, status)
            VALUES ($1, $2, $3, $4, $5, $6, 'pending')
            RETURNING id, inquiry_id, seller_id, buyer_id, quantity, unit_price, total_price, transaction_date, status
            "#
        )
        .bind(&request.inquiry_id)
        .bind(seller_id)
        .bind(buyer_id)
        .bind(request.quantity)
        .bind(request.unit_price)
        .bind(total_price)
        .fetch_one(&self.pool)
        .await?;

        Ok(Transaction {
            id: row.try_get("id")?,
            inquiry_id: row.try_get("inquiry_id")?,
            seller_id: row.try_get("seller_id")?,
            buyer_id: row.try_get("buyer_id")?,
            quantity: row.try_get("quantity")?,
            unit_price: row.try_get("unit_price")?,
            total_price: row.try_get("total_price")?,
            transaction_date: row.try_get("transaction_date")?,
            status: row.try_get("status")?,
        })
    }

    pub async fn find_transaction_by_id(&self, id: Uuid) -> Result<Option<Transaction>> {
        let row = query(
            "SELECT id, inquiry_id, seller_id, buyer_id, quantity, unit_price, total_price, transaction_date, status FROM transactions WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(Transaction {
                id: row.try_get("id")?,
                inquiry_id: row.try_get("inquiry_id")?,
                seller_id: row.try_get("seller_id")?,
                buyer_id: row.try_get("buyer_id")?,
                quantity: row.try_get("quantity")?,
                unit_price: row.try_get("unit_price")?,
                total_price: row.try_get("total_price")?,
                transaction_date: row.try_get("transaction_date")?,
                status: row.try_get("status")?,
            })),
            None => Ok(None),
        }
    }

    pub async fn get_transactions_for_user(&self, user_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Transaction>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let rows = query(
            "SELECT id, inquiry_id, seller_id, buyer_id, quantity, unit_price, total_price, transaction_date, status 
             FROM transactions WHERE seller_id = $1 OR buyer_id = $1 ORDER BY transaction_date DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut transactions = Vec::new();
        for row in rows {
            transactions.push(Transaction {
                id: row.try_get("id")?,
                inquiry_id: row.try_get("inquiry_id")?,
                seller_id: row.try_get("seller_id")?,
                buyer_id: row.try_get("buyer_id")?,
                quantity: row.try_get("quantity")?,
                unit_price: row.try_get("unit_price")?,
                total_price: row.try_get("total_price")?,
                transaction_date: row.try_get("transaction_date")?,
                status: row.try_get("status")?,
            });
        }

        Ok(transactions)
    }

    pub async fn update_transaction_status(&self, transaction_id: Uuid, status: &str) -> Result<Transaction> {
        let row = query(
            r#"
            UPDATE transactions SET status = $1
            WHERE id = $2
            RETURNING id, inquiry_id, seller_id, buyer_id, quantity, unit_price, total_price, transaction_date, status
            "#
        )
        .bind(status)
        .bind(transaction_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Transaction {
            id: row.try_get("id")?,
            inquiry_id: row.try_get("inquiry_id")?,
            seller_id: row.try_get("seller_id")?,
            buyer_id: row.try_get("buyer_id")?,
            quantity: row.try_get("quantity")?,
            unit_price: row.try_get("unit_price")?,
            total_price: row.try_get("total_price")?,
            transaction_date: row.try_get("transaction_date")?,
            status: row.try_get("status")?,
        })
    }

    pub async fn inquiry_exists_for_buyer(&self, inventory_id: Uuid, buyer_id: Uuid) -> Result<bool> {
        let row = query(
            "SELECT EXISTS(SELECT 1 FROM inquiries WHERE inventory_id = $1 AND buyer_id = $2 AND status IN ('pending', 'accepted')) as exists"
        )
        .bind(inventory_id)
        .bind(buyer_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get::<bool, _>("exists").unwrap_or(false))
    }

    pub async fn can_access_inquiry(&self, inquiry_id: Uuid, user_id: Uuid) -> Result<bool> {
        let row = query(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM inquiries i
                JOIN inventory inv ON i.inventory_id = inv.id
                WHERE i.id = $1 AND (i.buyer_id = $2 OR inv.user_id = $2)
            ) as can_access
            "#
        )
        .bind(inquiry_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get::<bool, _>("can_access").unwrap_or(false))
    }

    pub async fn can_access_transaction(&self, transaction_id: Uuid, user_id: Uuid) -> Result<bool> {
        let row = query(
            "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = $1 AND (seller_id = $2 OR buyer_id = $2)) as can_access"
        )
        .bind(transaction_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get::<bool, _>("can_access").unwrap_or(false))
    }
}
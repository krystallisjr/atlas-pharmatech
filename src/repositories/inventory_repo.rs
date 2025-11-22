use sqlx::{PgPool, query, Row};
use uuid::Uuid;
use chrono::Utc;
use crate::models::inventory::{Inventory, InventoryWithDetails, CreateInventoryRequest, UpdateInventoryRequest, SearchInventoryRequest};
use crate::middleware::error_handling::{Result, AppError};

pub struct InventoryRepository {
    pool: PgPool,
}

impl InventoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, request: &CreateInventoryRequest, user_id: Uuid) -> Result<Inventory> {
        let row = query(
            r#"
            INSERT INTO inventory (user_id, pharmaceutical_id, batch_number, quantity, expiry_date, unit_price, storage_location, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'available')
            RETURNING id, user_id, pharmaceutical_id, batch_number, quantity, expiry_date, unit_price, storage_location, status, created_at, updated_at
            "#
        )
        .bind(user_id)
        .bind(request.pharmaceutical_id)
        .bind(&request.batch_number)
        .bind(request.quantity)
        .bind(request.expiry_date)
        .bind(request.unit_price)
        .bind(&request.storage_location)
        .fetch_one(&self.pool)
        .await?;

        let inventory = Inventory {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            pharmaceutical_id: row.try_get("pharmaceutical_id")?,
            batch_number: row.try_get("batch_number")?,
            quantity: row.try_get("quantity")?,
            expiry_date: row.try_get("expiry_date")?,
            unit_price: row.try_get("unit_price")?,
            storage_location: row.try_get("storage_location")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        Ok(inventory)
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Inventory>> {
        let row = query(
            "SELECT id, user_id, pharmaceutical_id, batch_number, quantity, expiry_date, unit_price, storage_location, status, created_at, updated_at FROM inventory WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let inventory = Inventory {
                    id: row.try_get("id")?,
                    user_id: row.try_get("user_id")?,
                    pharmaceutical_id: row.try_get("pharmaceutical_id")?,
                    batch_number: row.try_get("batch_number")?,
                    quantity: row.try_get("quantity")?,
                    expiry_date: row.try_get("expiry_date")?,
                    unit_price: row.try_get("unit_price")?,
                    storage_location: row.try_get("storage_location")?,
                    status: row.try_get("status")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                };
                Ok(Some(inventory))
            }
            None => Ok(None),
        }
    }

    pub async fn find_by_user(&self, user_id: Uuid, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<Inventory>> {
        let limit = limit.unwrap_or(50).min(100);
        let offset = offset.unwrap_or(0);

        let rows = query(
            "SELECT id, user_id, pharmaceutical_id, batch_number, quantity, expiry_date, unit_price, storage_location, status, created_at, updated_at 
             FROM inventory WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let inventories = rows.into_iter()
            .map(|row| -> Result<Inventory> {
                Ok(Inventory {
                    id: row.try_get("id")?,
                    user_id: row.try_get("user_id")?,
                    pharmaceutical_id: row.try_get("pharmaceutical_id")?,
                    batch_number: row.try_get("batch_number")?,
                    quantity: row.try_get("quantity")?,
                    expiry_date: row.try_get("expiry_date")?,
                    unit_price: row.try_get("unit_price")?,
                    storage_location: row.try_get("storage_location")?,
                    status: row.try_get("status")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(inventories)
    }

    pub async fn search_with_details(&self, request: &SearchInventoryRequest) -> Result<Vec<InventoryWithDetails>> {
        let limit = request.limit.unwrap_or(50).min(100);
        let offset = request.offset.unwrap_or(0);

        // Use a simpler, production-ready approach with a well-structured query
        let mut query_str = r#"
            SELECT
                i.id, i.user_id, i.pharmaceutical_id, i.batch_number, i.quantity, i.expiry_date,
                i.unit_price, i.storage_location, i.status, i.created_at, i.updated_at,
                u.id as u_id, u.email, u.company_name, u.contact_person, u.phone, u.address, u.license_number, u.is_verified, u.role, u.created_at as user_created_at,
                p.id as pharma_id, p.brand_name, p.generic_name, p.ndc_code, p.manufacturer, p.category, p.description, p.strength, p.dosage_form, p.storage_requirements, p.created_at as pharma_created_at
            FROM inventory i
            JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id
            JOIN users u ON i.user_id = u.id
            WHERE i.status = 'available'
        "#.to_string();

        let mut params = Vec::new();
        let mut param_count = 0;

        // Add filters safely with parameter binding
        if let Some(pharma_id) = request.pharmaceutical_id {
            query_str.push_str(&format!(" AND i.pharmaceutical_id = ${}", param_count + 1));
            params.push(pharma_id.to_string());
            param_count += 1;
        }

        if let Some(ref brand_name) = request.brand_name {
            query_str.push_str(&format!(" AND p.brand_name ILIKE ${}", param_count + 1));
            params.push(format!("%{}%", brand_name));
            param_count += 1;
        }

        if let Some(ref generic_name) = request.generic_name {
            query_str.push_str(&format!(" AND p.generic_name ILIKE ${}", param_count + 1));
            params.push(format!("%{}%", generic_name));
            param_count += 1;
        }

        if let Some(ref manufacturer) = request.manufacturer {
            query_str.push_str(&format!(" AND p.manufacturer ILIKE ${}", param_count + 1));
            params.push(format!("%{}%", manufacturer));
            param_count += 1;
        }

        if let Some(ref ndc_code) = request.ndc_code {
            query_str.push_str(&format!(" AND p.ndc_code = ${}", param_count + 1));
            params.push(ndc_code.clone());
            param_count += 1;
        }

        if let Some(expiry_before) = request.expiry_before {
            query_str.push_str(&format!(" AND i.expiry_date <= ${}", param_count + 1));
            params.push(expiry_before.to_string());
            param_count += 1;
        }

        if let Some(expiry_after) = request.expiry_after {
            query_str.push_str(&format!(" AND i.expiry_date >= ${}", param_count + 1));
            params.push(expiry_after.to_string());
            param_count += 1;
        }

        if let Some(min_quantity) = request.min_quantity {
            query_str.push_str(&format!(" AND i.quantity >= ${}", param_count + 1));
            params.push(min_quantity.to_string());
            param_count += 1;
        }

        if let Some(max_quantity) = request.max_quantity {
            query_str.push_str(&format!(" AND i.quantity <= ${}", param_count + 1));
            params.push(max_quantity.to_string());
            param_count += 1;
        }

        if let Some(ref status) = request.status {
            query_str.push_str(&format!(" AND i.status = ${}", param_count + 1));
            params.push(status.clone());
            param_count += 1;
        }

        if let Some(min_price) = request.min_price {
            query_str.push_str(&format!(" AND i.unit_price >= ${}", param_count + 1));
            params.push(min_price.to_string());
            param_count += 1;
        }

        if let Some(max_price) = request.max_price {
            query_str.push_str(&format!(" AND i.unit_price <= ${}", param_count + 1));
            params.push(max_price.to_string());
            param_count += 1;
        }

        // Add ordering and pagination
        let sort_by = request.sort_by.as_deref().unwrap_or("expiry_date");
        let sort_order = request.sort_order.as_deref().unwrap_or("asc");
        query_str.push_str(&format!(" ORDER BY i.{} {} LIMIT {} OFFSET {}", sort_by, sort_order, limit, offset));

        // Execute the query with proper parameter binding
        let mut query_builder = query(&query_str);
        for param in params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        // Process results with explicit error handling
        let mut results = Vec::new();
        for row in rows {
            // Extract inventory data with proper error handling
            let inventory = Inventory {
                id: row.try_get("id")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get inventory id: {}", e)))?,
                user_id: row.try_get("user_id")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get user_id: {}", e)))?,
                pharmaceutical_id: row.try_get("pharmaceutical_id")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get pharmaceutical_id: {}", e)))?,
                batch_number: row.try_get("batch_number")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get batch_number: {}", e)))?,
                quantity: row.try_get("quantity")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get quantity: {}", e)))?,
                expiry_date: row.try_get("expiry_date")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get expiry_date: {}", e)))?,
                unit_price: row.try_get("unit_price")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get unit_price: {}", e)))?,
                storage_location: row.try_get("storage_location")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get storage_location: {}", e)))?,
                status: row.try_get("status")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get status: {}", e)))?,
                created_at: row.try_get("created_at")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get created_at: {}", e)))?,
                updated_at: row.try_get("updated_at")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get updated_at: {}", e)))?,
            };

            // Extract user data
            let user = crate::models::user::UserResponse {
                id: row.try_get("u_id")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get user id: {}", e)))?,
                email: row.try_get("email")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get email: {}", e)))?,
                company_name: row.try_get("company_name")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get company_name: {}", e)))?,
                contact_person: row.try_get("contact_person")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get contact_person: {}", e)))?,
                phone: row.try_get("phone")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get phone: {}", e)))?,
                address: row.try_get("address")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get address: {}", e)))?,
                license_number: row.try_get("license_number")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get license_number: {}", e)))?,
                is_verified: row.try_get("is_verified")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get is_verified: {}", e)))?,
                role: row.try_get("role")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get role: {}", e)))?,
                created_at: row.try_get("user_created_at")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get user_created_at: {}", e)))?,
            };

            // Extract pharmaceutical data
            let pharmaceutical = crate::models::pharmaceutical::PharmaceuticalResponse {
                id: row.try_get("pharma_id")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get pharma_id: {}", e)))?,
                brand_name: row.try_get("brand_name")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get brand_name: {}", e)))?,
                generic_name: row.try_get("generic_name")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get generic_name: {}", e)))?,
                ndc_code: row.try_get("ndc_code")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get ndc_code: {}", e)))?,
                manufacturer: row.try_get("manufacturer")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get manufacturer: {}", e)))?,
                category: row.try_get("category")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get category: {}", e)))?,
                description: row.try_get("description")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get description: {}", e)))?,
                strength: row.try_get("strength")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get strength: {}", e)))?,
                dosage_form: row.try_get("dosage_form")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get dosage_form: {}", e)))?,
                storage_requirements: row.try_get("storage_requirements")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get storage_requirements: {}", e)))?,
                created_at: row.try_get("pharma_created_at")
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to get pharma_created_at: {}", e)))?,
            };

            // Calculate days to expiry
            let days_to_expiry = inventory.expiry_date.signed_duration_since(chrono::Utc::now().date_naive()).num_days();

            // Build the response
            results.push(InventoryWithDetails {
                inventory: inventory.clone(),
                pharmaceutical: pharmaceutical.clone(),
                user: user.clone(),
            });
        }

        Ok(results)
    }

    pub async fn update(&self, inventory_id: Uuid, user_id: Uuid, request: &UpdateInventoryRequest) -> Result<Inventory> {
        // Build the SQL dynamically based on which fields are being updated
        use sqlx::QueryBuilder;

        let mut query_builder: QueryBuilder<sqlx::Postgres> = QueryBuilder::new("UPDATE inventory SET ");

        let mut has_fields = false;

        if let Some(quantity) = request.quantity {
            if has_fields {
                query_builder.push(", ");
            }
            query_builder.push("quantity = ");
            query_builder.push_bind(quantity);
            has_fields = true;
        }

        if let Some(expiry_date) = request.expiry_date {
            if has_fields {
                query_builder.push(", ");
            }
            query_builder.push("expiry_date = ");
            query_builder.push_bind(expiry_date);
            has_fields = true;
        }

        if let Some(unit_price) = request.unit_price {
            if has_fields {
                query_builder.push(", ");
            }
            query_builder.push("unit_price = ");
            query_builder.push_bind(unit_price);
            has_fields = true;
        }

        if let Some(ref storage_location) = request.storage_location {
            if has_fields {
                query_builder.push(", ");
            }
            query_builder.push("storage_location = ");
            query_builder.push_bind(storage_location);
            has_fields = true;
        }

        if let Some(ref status) = request.status {
            if has_fields {
                query_builder.push(", ");
            }
            query_builder.push("status = ");
            query_builder.push_bind(status);
            has_fields = true;
        }

        if !has_fields {
            // No updates to make, return existing inventory
            return self.find_by_id(inventory_id).await?
                .ok_or(AppError::NotFound("Resource not found".to_string()));
        }

        // Always update the timestamp
        query_builder.push(", updated_at = CURRENT_TIMESTAMP");

        // Add WHERE clause
        query_builder.push(" WHERE id = ");
        query_builder.push_bind(inventory_id);
        query_builder.push(" AND user_id = ");
        query_builder.push_bind(user_id);

        // Add RETURNING clause
        query_builder.push(" RETURNING id, user_id, pharmaceutical_id, batch_number, quantity, expiry_date, unit_price, storage_location, status, created_at, updated_at");

        let row = query_builder
            .build()
            .fetch_one(&self.pool)
            .await?;

        let inventory = Inventory {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            pharmaceutical_id: row.try_get("pharmaceutical_id")?,
            batch_number: row.try_get("batch_number")?,
            quantity: row.try_get("quantity")?,
            expiry_date: row.try_get("expiry_date")?,
            unit_price: row.try_get("unit_price")?,
            storage_location: row.try_get("storage_location")?,
            status: row.try_get("status")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        };

        Ok(inventory)
    }

    pub async fn delete(&self, inventory_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = query("DELETE FROM inventory WHERE id = $1 AND user_id = $2")
            .bind(inventory_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Resource not found".to_string()));
        }

        Ok(())
    }

    pub async fn get_expiry_alerts(&self, days_threshold: i64) -> Result<Vec<InventoryWithDetails>> {
        let threshold_date = Utc::now().date_naive() + chrono::Duration::days(days_threshold);

        // Use the same search_with_details logic but with expiry filtering
        let expiry_request = SearchInventoryRequest {
            pharmaceutical_id: None,
            brand_name: None,
            generic_name: None,
            manufacturer: None,
            ndc_code: None,
            expiry_before: Some(threshold_date),
            expiry_after: Some(Utc::now().date_naive()),
            min_quantity: None,
            max_quantity: None,
            status: Some("available".to_string()),
            min_price: None,
            max_price: None,
            limit: Some(1000), // High limit for alerts
            offset: Some(0),
            sort_by: Some("expiry_date".to_string()),
            sort_order: Some("asc".to_string()),
        };

        self.search_with_details(&expiry_request).await
    }

    pub async fn batch_exists(&self, user_id: Uuid, pharmaceutical_id: Uuid, batch_number: &str) -> Result<bool> {
        let row = query("SELECT EXISTS(SELECT 1 FROM inventory WHERE user_id = $1 AND pharmaceutical_id = $2 AND batch_number = $3) as exists")
            .bind(user_id)
            .bind(pharmaceutical_id)
            .bind(batch_number)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.try_get::<bool, _>("exists")?)
    }

    /// Update only the quantity of an inventory item (for ERP sync)
    pub async fn update_quantity(&self, inventory_id: Uuid, new_quantity: i32) -> Result<()> {
        query("UPDATE inventory SET quantity = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_quantity)
            .bind(inventory_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
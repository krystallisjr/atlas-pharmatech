/// Alert Scheduler Service
///
/// Background service responsible for periodically checking inventory
/// and marketplace conditions to generate automated alerts.
///
/// Checks performed:
/// - Expiry alerts (products expiring soon)
/// - Low stock alerts (inventory below threshold)
/// - Watchlist matches (new marketplace listings)

use crate::{
    middleware::error_handling::Result,
    models::alerts::*,
    models::inventory::SearchInventoryRequest,
    services::{NotificationService, InventoryService},
};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct AlertSchedulerService {
    db_pool: PgPool,
    notification_service: NotificationService,
    inventory_service: InventoryService,
}

impl AlertSchedulerService {
    pub fn new(db_pool: PgPool) -> Self {
        let notification_service = NotificationService::new(db_pool.clone());
        let inventory_repo = crate::repositories::InventoryRepository::new(db_pool.clone());
        let pharma_repo = crate::repositories::PharmaceuticalRepository::new(db_pool.clone());
        let inventory_service = InventoryService::new(inventory_repo, pharma_repo);

        Self {
            db_pool,
            notification_service,
            inventory_service,
        }
    }

    // ========================================================================
    // MAIN SCHEDULER ENTRY POINT
    // ========================================================================

    /// Run all scheduled alert checks
    pub async fn run_scheduled_checks(&self) -> Result<ScheduledRunStats> {
        let run_id = self.start_processing_log("scheduled_run").await?;
        let mut stats = ScheduledRunStats::default();

        tracing::info!("Starting scheduled alert checks: run_id={}", run_id);

        // Run checks in parallel for efficiency
        let (expiry_stats, stock_stats, watchlist_stats) = tokio::join!(
            self.check_expiry_alerts(),
            self.check_low_stock_alerts(),
            self.check_watchlist_alerts()
        );

        // Aggregate statistics
        if let Ok(expiry) = expiry_stats {
            stats.expiry_alerts_generated = expiry;
        } else {
            stats.errors_encountered += 1;
            tracing::error!("Expiry alert check failed: {:?}", expiry_stats);
        }

        if let Ok(stock) = stock_stats {
            stats.low_stock_alerts_generated = stock;
        } else {
            stats.errors_encountered += 1;
            tracing::error!("Low stock check failed: {:?}", stock_stats);
        }

        if let Ok(watchlist) = watchlist_stats {
            stats.watchlist_alerts_generated = watchlist;
        } else {
            stats.errors_encountered += 1;
            tracing::error!("Watchlist check failed: {:?}", watchlist_stats);
        }

        stats.total_alerts_generated = stats.expiry_alerts_generated
            + stats.low_stock_alerts_generated
            + stats.watchlist_alerts_generated;

        // Complete the processing log
        self.complete_processing_log(
            run_id,
            if stats.errors_encountered == 0 { "completed" } else { "failed" },
            stats.total_alerts_generated,
            stats.errors_encountered,
            None,
        ).await?;

        tracing::info!(
            "Scheduled alert checks completed: run_id={}, alerts={}, errors={}",
            run_id,
            stats.total_alerts_generated,
            stats.errors_encountered
        );

        Ok(stats)
    }

    // ========================================================================
    // EXPIRY ALERTS
    // ========================================================================

    /// Check for inventory items expiring soon and create alerts
    pub async fn check_expiry_alerts(&self) -> Result<i32> {
        let run_id = self.start_processing_log("expiry_check").await?;
        let mut alerts_created = 0;

        tracing::info!("Starting expiry alert check: run_id={}", run_id);

        // Get all users with expiry alerts enabled
        let users = sqlx::query!(
            r#"
            SELECT user_id, expiry_alert_days
            FROM user_alert_preferences
            WHERE expiry_alerts_enabled = TRUE AND in_app_notifications_enabled = TRUE
            "#
        )
        .fetch_all(&self.db_pool)
        .await?;

        for user_prefs in users {
            let user_id = user_prefs.user_id;
            let threshold_days = user_prefs.expiry_alert_days as i64;

            // Get expiring inventory for this user
            let threshold_date = Utc::now().date_naive() + chrono::Duration::days(threshold_days);

            let expiring_items = sqlx::query!(
                r#"
                SELECT
                    i.id,
                    i.quantity,
                    i.expiry_date,
                    p.brand_name || ' ' || p.generic_name as product_name,
                    (i.expiry_date - CURRENT_DATE) as days_to_expiry
                FROM inventory i
                JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id
                WHERE i.user_id = $1
                  AND i.status = 'available'
                  AND i.expiry_date > CURRENT_DATE
                  AND i.expiry_date <= $2
                  AND NOT EXISTS (
                      SELECT 1 FROM alert_notifications
                      WHERE user_id = $1
                        AND inventory_id = i.id
                        AND alert_type IN ('expiry_warning', 'expiry_critical')
                        AND created_at > NOW() - INTERVAL '7 days'
                  )
                "#,
                user_id,
                threshold_date
            )
            .fetch_all(&self.db_pool)
            .await?;

            // Create alerts for each expiring item
            for item in expiring_items {
                let days_to_expiry = item.days_to_expiry.unwrap_or(0) as i64;
                let product_name = item.product_name.unwrap_or_else(|| "Unknown Product".to_string());

                let payload = AlertPayload::new_expiry_warning(
                    user_id,
                    item.id,
                    &product_name,
                    days_to_expiry,
                    item.quantity,
                );

                match self.notification_service.create_alert(payload).await {
                    Ok(_) => {
                        alerts_created += 1;
                        tracing::debug!(
                            "Expiry alert created: user={}, product={}, days={}",
                            user_id,
                            product_name,
                            days_to_expiry
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to create expiry alert: {}", e);
                    }
                }
            }
        }

        self.complete_processing_log(run_id, "completed", alerts_created, 0, None).await?;

        tracing::info!("Expiry alert check completed: {} alerts created", alerts_created);

        Ok(alerts_created)
    }

    // ========================================================================
    // LOW STOCK ALERTS
    // ========================================================================

    /// Check for low stock inventory and create alerts
    pub async fn check_low_stock_alerts(&self) -> Result<i32> {
        let run_id = self.start_processing_log("low_stock_check").await?;
        let mut alerts_created = 0;

        tracing::info!("Starting low stock alert check: run_id={}", run_id);

        // Get all users with low stock alerts enabled
        let users = sqlx::query!(
            r#"
            SELECT user_id, low_stock_threshold
            FROM user_alert_preferences
            WHERE low_stock_alerts_enabled = TRUE AND in_app_notifications_enabled = TRUE
            "#
        )
        .fetch_all(&self.db_pool)
        .await?;

        tracing::info!("Found {} users with low stock alerts enabled", users.len());

        for user_prefs in users {
            let user_id = user_prefs.user_id;
            let threshold = user_prefs.low_stock_threshold;

            tracing::info!("Checking low stock for user {} with threshold {}", user_id, threshold);

            // Get low stock items for this user
            let low_stock_items = sqlx::query!(
                r#"
                SELECT
                    i.id,
                    i.quantity,
                    p.brand_name || ' ' || p.generic_name as product_name
                FROM inventory i
                JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id
                WHERE i.user_id = $1
                  AND i.status = 'available'
                  AND i.quantity > 0
                  AND i.quantity < $2
                  AND NOT EXISTS (
                      SELECT 1 FROM alert_notifications
                      WHERE user_id = $1
                        AND inventory_id = i.id
                        AND alert_type = 'low_stock'
                        AND created_at > NOW() - INTERVAL '7 days'
                  )
                "#,
                user_id,
                threshold
            )
            .fetch_all(&self.db_pool)
            .await?;

            tracing::info!("Found {} low stock items for user {}", low_stock_items.len(), user_id);

            // Create alerts for each low stock item
            for item in low_stock_items {
                let product_name = item.product_name.unwrap_or_else(|| "Unknown Product".to_string());

                let payload = AlertPayload::new_low_stock(
                    user_id,
                    item.id,
                    &product_name,
                    item.quantity,
                    threshold,
                );

                match self.notification_service.create_alert(payload).await {
                    Ok(_) => {
                        alerts_created += 1;
                        tracing::debug!(
                            "Low stock alert created: user={}, product={}, qty={}",
                            user_id,
                            product_name,
                            item.quantity
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to create low stock alert: {}", e);
                    }
                }
            }
        }

        self.complete_processing_log(run_id, "completed", alerts_created, 0, None).await?;

        tracing::info!("Low stock alert check completed: {} alerts created", alerts_created);

        Ok(alerts_created)
    }

    // ========================================================================
    // WATCHLIST ALERTS
    // ========================================================================

    /// Check marketplace watchlists for new matches
    pub async fn check_watchlist_alerts(&self) -> Result<i32> {
        let run_id = self.start_processing_log("watchlist_check").await?;
        let mut alerts_created = 0;

        tracing::info!("Starting watchlist alert check: run_id={}", run_id);

        // Get all active watchlists
        let watchlists = sqlx::query_as!(
            MarketplaceWatchlist,
            r#"
            SELECT w.*
            FROM marketplace_watchlist w
            JOIN user_alert_preferences p ON w.user_id = p.user_id
            WHERE w.alert_enabled = TRUE
              AND p.watchlist_alerts_enabled = TRUE
              AND p.in_app_notifications_enabled = TRUE
            "#
        )
        .fetch_all(&self.db_pool)
        .await?;

        for watchlist in watchlists {
            // Extract search criteria from JSONB and count matching marketplace items
            let criteria = &watchlist.search_criteria;
            let search_term = criteria.get("search_term").and_then(|v| v.as_str()).map(|s| format!("%{}%", s));

            // Simple count query - just check if new items match the basic criteria
            let match_count_result = sqlx::query!(
                r#"
                SELECT COUNT(*)::INT as "count!"
                FROM inventory i
                JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id
                WHERE i.status = 'available'
                  AND i.user_id != $1
                  AND ($2::TEXT IS NULL OR
                       p.brand_name ILIKE $2 OR
                       p.generic_name ILIKE $2 OR
                       p.manufacturer ILIKE $2)
                "#,
                watchlist.user_id,
                search_term
            )
            .fetch_one(&self.db_pool)
            .await;

            let match_count = match match_count_result {
                Ok(record) => record.count,
                Err(e) => {
                    tracing::error!("Watchlist query failed for {}: {}", watchlist.id, e);
                    continue;
                }
            };

            // Only create alert if there are new matches and count has increased
            if match_count > 0 && match_count > watchlist.last_match_count {
                let new_match_count = match_count - watchlist.last_match_count;

                // Get first inventory ID for the alert
                let first_inventory_id = sqlx::query!(
                    r#"
                    SELECT i.id
                    FROM inventory i
                    JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id
                    WHERE i.status = 'available'
                      AND i.user_id != $1
                      AND ($2::TEXT IS NULL OR
                           p.brand_name ILIKE $2 OR
                           p.generic_name ILIKE $2 OR
                           p.manufacturer ILIKE $2)
                    ORDER BY i.created_at DESC
                    LIMIT 1
                    "#,
                    watchlist.user_id,
                    search_term
                )
                .fetch_optional(&self.db_pool)
                .await
                .ok()
                .flatten()
                .map(|r| r.id);

                let payload = AlertPayload::new_watchlist_match(
                    watchlist.user_id,
                    &watchlist.name,
                    new_match_count,
                    first_inventory_id,
                );

                match self.notification_service.create_alert(payload).await {
                    Ok(_) => {
                        alerts_created += 1;
                        tracing::debug!(
                            "Watchlist alert created: user={}, watchlist={}, matches={}",
                            watchlist.user_id,
                            watchlist.name,
                            new_match_count
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to create watchlist alert: {}", e);
                    }
                }
            }

            // Update watchlist statistics
            if let Err(e) = self.notification_service.update_watchlist_stats(watchlist.id, match_count).await {
                tracing::error!("Failed to update watchlist stats: {}", e);
            }
        }

        self.complete_processing_log(run_id, "completed", alerts_created, 0, None).await?;

        tracing::info!("Watchlist alert check completed: {} alerts created", alerts_created);

        Ok(alerts_created)
    }

    // ========================================================================
    // PROCESSING LOG HELPERS
    // ========================================================================

    async fn start_processing_log(&self, run_type: &str) -> Result<Uuid> {
        let log = sqlx::query!(
            r#"
            INSERT INTO alert_processing_log (run_type, status)
            VALUES ($1, 'running')
            RETURNING id
            "#,
            run_type
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(log.id)
    }

    async fn complete_processing_log(
        &self,
        log_id: Uuid,
        status: &str,
        alerts_generated: i32,
        errors_encountered: i32,
        error_details: Option<String>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE alert_processing_log
            SET completed_at = NOW(),
                status = $2,
                alerts_generated = $3,
                errors_encountered = $4,
                error_details = $5
            WHERE id = $1
            "#,
            log_id,
            status,
            alerts_generated,
            errors_encountered,
            error_details
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}

// ============================================================================
// STATISTICS STRUCTURE
// ============================================================================

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ScheduledRunStats {
    pub expiry_alerts_generated: i32,
    pub low_stock_alerts_generated: i32,
    pub watchlist_alerts_generated: i32,
    pub total_alerts_generated: i32,
    pub errors_encountered: i32,
}

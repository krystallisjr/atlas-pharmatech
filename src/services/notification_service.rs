/// Notification Service
///
/// Handles creation, retrieval, and management of user notifications and alerts.
/// This service is responsible for:
/// - Creating new alert notifications
/// - Fetching user notifications with filtering
/// - Marking notifications as read/dismissed
/// - Managing user alert preferences
/// - Managing marketplace watchlists

use crate::{
    middleware::error_handling::{Result, AppError},
    models::alerts::*,
};
use sqlx::PgPool;
use uuid::Uuid;

pub struct NotificationService {
    db_pool: PgPool,
}

impl NotificationService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    // ========================================================================
    // ALERT NOTIFICATION CRUD
    // ========================================================================

    /// Create a new alert notification from payload
    pub async fn create_alert(&self, payload: AlertPayload) -> Result<AlertNotification> {
        let notification = sqlx::query_as!(
            AlertNotification,
            r#"
            INSERT INTO alert_notifications (
                user_id, alert_type, severity, title, message,
                inventory_id, related_user_id, metadata, action_url
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
            payload.user_id,
            payload.alert_type.as_str(),
            payload.severity.as_str(),
            payload.title,
            payload.message,
            payload.inventory_id,
            payload.related_user_id,
            payload.metadata,
            payload.action_url
        )
        .fetch_one(&self.db_pool)
        .await?;

        tracing::info!(
            "Alert created: type={}, user={}, severity={}",
            notification.alert_type,
            notification.user_id,
            notification.severity
        );

        Ok(notification)
    }

    /// Get notifications for a user with optional filtering
    pub async fn get_user_notifications(
        &self,
        user_id: Uuid,
        query: GetNotificationsQuery,
    ) -> Result<NotificationSummary> {
        let limit = query.limit.unwrap_or(50).min(100);
        let offset = query.offset.unwrap_or(0);

        // Build query conditionally
        let mut base_query = String::from(
            "SELECT * FROM alert_notifications WHERE user_id = $1 AND is_dismissed = FALSE"
        );

        if query.unread_only == Some(true) {
            base_query.push_str(" AND is_read = FALSE");
        }

        if let Some(ref alert_type) = query.alert_type {
            base_query.push_str(&format!(" AND alert_type = '{}'", alert_type));
        }

        base_query.push_str(" ORDER BY created_at DESC LIMIT $2 OFFSET $3");

        let notifications = sqlx::query_as::<_, AlertNotification>(&base_query)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db_pool)
            .await?;

        // Get total counts
        let total_unread: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM alert_notifications WHERE user_id = $1 AND is_read = FALSE AND is_dismissed = FALSE",
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?
        .unwrap_or(0);

        let total_notifications: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM alert_notifications WHERE user_id = $1 AND is_dismissed = FALSE",
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?
        .unwrap_or(0);

        Ok(NotificationSummary {
            total_unread,
            total_notifications,
            notifications: notifications.into_iter().map(Into::into).collect(),
        })
    }

    /// Mark a notification as read
    pub async fn mark_as_read(&self, notification_id: Uuid, user_id: Uuid, is_read: bool) -> Result<()> {
        let result = sqlx::query!(
            "UPDATE alert_notifications SET is_read = $1 WHERE id = $2 AND user_id = $3",
            is_read,
            notification_id,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Notification not found".to_string()));
        }

        Ok(())
    }

    /// Mark all notifications as read for a user
    pub async fn mark_all_read(&self, user_id: Uuid) -> Result<u64> {
        let result = sqlx::query!(
            "UPDATE alert_notifications SET is_read = TRUE WHERE user_id = $1 AND is_read = FALSE",
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Dismiss a notification (soft delete)
    pub async fn dismiss_notification(&self, notification_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            "UPDATE alert_notifications SET is_dismissed = TRUE WHERE id = $1 AND user_id = $2",
            notification_id,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Notification not found".to_string()));
        }

        Ok(())
    }

    /// Get unread notification count
    pub async fn get_unread_count(&self, user_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar!(
            "SELECT get_unread_alert_count($1)",
            user_id
        )
        .fetch_one(&self.db_pool)
        .await?
        .unwrap_or(0);

        Ok(count as i64)
    }

    // ========================================================================
    // USER ALERT PREFERENCES
    // ========================================================================

    /// Get user's alert preferences (creates default if not exists)
    pub async fn get_user_preferences(&self, user_id: Uuid) -> Result<UserAlertPreferences> {
        let prefs = sqlx::query_as!(
            UserAlertPreferences,
            "SELECT * FROM user_alert_preferences WHERE user_id = $1",
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        match prefs {
            Some(p) => Ok(p),
            None => {
                // Create default preferences
                let new_prefs = sqlx::query_as!(
                    UserAlertPreferences,
                    r#"
                    INSERT INTO user_alert_preferences (user_id)
                    VALUES ($1)
                    RETURNING *
                    "#,
                    user_id
                )
                .fetch_one(&self.db_pool)
                .await?;

                Ok(new_prefs)
            }
        }
    }

    /// Update user's alert preferences
    pub async fn update_user_preferences(
        &self,
        user_id: Uuid,
        update: UpdateAlertPreferencesRequest,
    ) -> Result<UserAlertPreferences> {
        // Build dynamic update query
        let mut updates = Vec::new();
        let mut param_count = 1;

        if update.expiry_alerts_enabled.is_some() {
            param_count += 1;
            updates.push(format!("expiry_alerts_enabled = ${}", param_count));
        }
        if update.expiry_alert_days.is_some() {
            param_count += 1;
            updates.push(format!("expiry_alert_days = ${}", param_count));
        }
        if update.low_stock_alerts_enabled.is_some() {
            param_count += 1;
            updates.push(format!("low_stock_alerts_enabled = ${}", param_count));
        }
        if update.low_stock_threshold.is_some() {
            param_count += 1;
            updates.push(format!("low_stock_threshold = ${}", param_count));
        }
        if update.watchlist_alerts_enabled.is_some() {
            param_count += 1;
            updates.push(format!("watchlist_alerts_enabled = ${}", param_count));
        }
        if update.email_notifications_enabled.is_some() {
            param_count += 1;
            updates.push(format!("email_notifications_enabled = ${}", param_count));
        }
        if update.in_app_notifications_enabled.is_some() {
            param_count += 1;
            updates.push(format!("in_app_notifications_enabled = ${}", param_count));
        }

        if updates.is_empty() {
            return self.get_user_preferences(user_id).await;
        }

        let query = format!(
            "UPDATE user_alert_preferences SET {} WHERE user_id = $1 RETURNING *",
            updates.join(", ")
        );

        // Execute with dynamic parameters
        let mut query_builder = sqlx::query_as::<_, UserAlertPreferences>(&query);
        query_builder = query_builder.bind(user_id);

        if let Some(val) = update.expiry_alerts_enabled {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.expiry_alert_days {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.low_stock_alerts_enabled {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.low_stock_threshold {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.watchlist_alerts_enabled {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.email_notifications_enabled {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.in_app_notifications_enabled {
            query_builder = query_builder.bind(val);
        }

        let updated = query_builder.fetch_one(&self.db_pool).await?;

        tracing::info!("Alert preferences updated for user: {}", user_id);

        Ok(updated)
    }

    // ========================================================================
    // MARKETPLACE WATCHLIST
    // ========================================================================

    /// Create a new watchlist item
    pub async fn create_watchlist(
        &self,
        user_id: Uuid,
        request: CreateWatchlistRequest,
    ) -> Result<MarketplaceWatchlist> {
        let watchlist = sqlx::query_as!(
            MarketplaceWatchlist,
            r#"
            INSERT INTO marketplace_watchlist (user_id, name, description, search_criteria, alert_enabled)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
            user_id,
            request.name,
            request.description,
            request.search_criteria,
            request.alert_enabled.unwrap_or(true)
        )
        .fetch_one(&self.db_pool)
        .await?;

        tracing::info!("Watchlist created: {} for user: {}", watchlist.name, user_id);

        Ok(watchlist)
    }

    /// Get all watchlists for a user
    pub async fn get_user_watchlists(&self, user_id: Uuid) -> Result<Vec<MarketplaceWatchlist>> {
        let watchlists = sqlx::query_as!(
            MarketplaceWatchlist,
            "SELECT * FROM marketplace_watchlist WHERE user_id = $1 ORDER BY created_at DESC",
            user_id
        )
        .fetch_all(&self.db_pool)
        .await?;

        Ok(watchlists)
    }

    /// Get watchlist by ID
    pub async fn get_watchlist(&self, watchlist_id: Uuid, user_id: Uuid) -> Result<MarketplaceWatchlist> {
        let watchlist = sqlx::query_as!(
            MarketplaceWatchlist,
            "SELECT * FROM marketplace_watchlist WHERE id = $1 AND user_id = $2",
            watchlist_id,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Watchlist not found".to_string()))?;

        Ok(watchlist)
    }

    /// Update a watchlist
    pub async fn update_watchlist(
        &self,
        watchlist_id: Uuid,
        user_id: Uuid,
        update: UpdateWatchlistRequest,
    ) -> Result<MarketplaceWatchlist> {
        // Verify ownership
        self.get_watchlist(watchlist_id, user_id).await?;

        let mut updates = Vec::new();
        let mut param_count = 2; // id and user_id

        if update.name.is_some() {
            param_count += 1;
            updates.push(format!("name = ${}", param_count));
        }
        if update.description.is_some() {
            param_count += 1;
            updates.push(format!("description = ${}", param_count));
        }
        if update.search_criteria.is_some() {
            param_count += 1;
            updates.push(format!("search_criteria = ${}", param_count));
        }
        if update.alert_enabled.is_some() {
            param_count += 1;
            updates.push(format!("alert_enabled = ${}", param_count));
        }

        if updates.is_empty() {
            return self.get_watchlist(watchlist_id, user_id).await;
        }

        let query = format!(
            "UPDATE marketplace_watchlist SET {} WHERE id = $1 AND user_id = $2 RETURNING *",
            updates.join(", ")
        );

        let mut query_builder = sqlx::query_as::<_, MarketplaceWatchlist>(&query);
        query_builder = query_builder.bind(watchlist_id).bind(user_id);

        if let Some(val) = update.name {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.description {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.search_criteria {
            query_builder = query_builder.bind(val);
        }
        if let Some(val) = update.alert_enabled {
            query_builder = query_builder.bind(val);
        }

        let updated = query_builder.fetch_one(&self.db_pool).await?;

        tracing::info!("Watchlist updated: {}", watchlist_id);

        Ok(updated)
    }

    /// Delete a watchlist
    pub async fn delete_watchlist(&self, watchlist_id: Uuid, user_id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            "DELETE FROM marketplace_watchlist WHERE id = $1 AND user_id = $2",
            watchlist_id,
            user_id
        )
        .execute(&self.db_pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Watchlist not found".to_string()));
        }

        tracing::info!("Watchlist deleted: {}", watchlist_id);

        Ok(())
    }

    /// Update watchlist last checked timestamp and match count
    pub async fn update_watchlist_stats(
        &self,
        watchlist_id: Uuid,
        new_matches: i32,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE marketplace_watchlist
            SET last_checked_at = NOW(),
                last_match_count = $1,
                total_matches_found = total_matches_found + $1
            WHERE id = $2
            "#,
            new_matches,
            watchlist_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }
}

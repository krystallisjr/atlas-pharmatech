/// Alert System REST API Handlers
///
/// HTTP endpoints for alert notifications, preferences, and watchlist management.

use axum::{
    extract::{State, Path, Query},
    Extension,
    Json,
};
use uuid::Uuid;
use crate::{
    config::AppConfig,
    middleware::{error_handling::Result, Claims},
    models::alerts::*,
    services::NotificationService,
};

// ============================================================================
// NOTIFICATION ENDPOINTS
// ============================================================================

/// GET /api/alerts/notifications
/// Get user's notifications with optional filtering
pub async fn get_notifications(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<GetNotificationsQuery>,
) -> Result<Json<NotificationSummary>> {
    let service = NotificationService::new(config.database_pool.clone());
    let notifications = service.get_user_notifications(claims.user_id, query).await?;

    Ok(Json(notifications))
}

/// GET /api/alerts/notifications/unread-count
/// Get count of unread notifications
pub async fn get_unread_count(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>> {
    let service = NotificationService::new(config.database_pool.clone());
    let count = service.get_unread_count(claims.user_id).await?;

    Ok(Json(serde_json::json!({
        "unread_count": count
    })))
}

/// PUT /api/alerts/notifications/:id/read
/// Mark a notification as read/unread
pub async fn mark_notification_read(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(notification_id): Path<Uuid>,
    Json(request): Json<MarkAlertReadRequest>,
) -> Result<Json<serde_json::Value>> {
    let service = NotificationService::new(config.database_pool.clone());
    service.mark_as_read(notification_id, claims.user_id, request.is_read).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification updated"
    })))
}

/// POST /api/alerts/notifications/mark-all-read
/// Mark all notifications as read for the user
pub async fn mark_all_read(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>> {
    let service = NotificationService::new(config.database_pool.clone());
    let count = service.mark_all_read(claims.user_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "marked_read": count,
        "message": format!("{} notifications marked as read", count)
    })))
}

/// DELETE /api/alerts/notifications/:id
/// Dismiss (soft delete) a notification
pub async fn dismiss_notification(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let service = NotificationService::new(config.database_pool.clone());
    service.dismiss_notification(notification_id, claims.user_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Notification dismissed"
    })))
}

// ============================================================================
// ALERT PREFERENCES ENDPOINTS
// ============================================================================

/// GET /api/alerts/preferences
/// Get user's alert preferences
pub async fn get_preferences(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserAlertPreferences>> {
    let service = NotificationService::new(config.database_pool.clone());
    let preferences = service.get_user_preferences(claims.user_id).await?;

    Ok(Json(preferences))
}

/// PUT /api/alerts/preferences
/// Update user's alert preferences
pub async fn update_preferences(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<UpdateAlertPreferencesRequest>,
) -> Result<Json<UserAlertPreferences>> {
    tracing::info!(
        "Updating alert preferences for user: {}",
        claims.user_id
    );

    let service = NotificationService::new(config.database_pool.clone());
    let updated = service.update_user_preferences(claims.user_id, request).await?;

    Ok(Json(updated))
}

// ============================================================================
// WATCHLIST ENDPOINTS
// ============================================================================

/// GET /api/alerts/watchlist
/// Get all watchlists for the user
pub async fn get_watchlists(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<WatchlistResponse>>> {
    let service = NotificationService::new(config.database_pool.clone());
    let watchlists = service.get_user_watchlists(claims.user_id).await?;

    let response: Vec<WatchlistResponse> = watchlists.into_iter().map(Into::into).collect();

    Ok(Json(response))
}

/// POST /api/alerts/watchlist
/// Create a new watchlist
pub async fn create_watchlist(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreateWatchlistRequest>,
) -> Result<Json<WatchlistResponse>> {
    tracing::info!(
        "Creating watchlist '{}' for user: {}",
        request.name,
        claims.user_id
    );

    let service = NotificationService::new(config.database_pool.clone());
    let watchlist = service.create_watchlist(claims.user_id, request).await?;

    Ok(Json(watchlist.into()))
}

/// GET /api/alerts/watchlist/:id
/// Get a specific watchlist
pub async fn get_watchlist(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(watchlist_id): Path<Uuid>,
) -> Result<Json<WatchlistResponse>> {
    let service = NotificationService::new(config.database_pool.clone());
    let watchlist = service.get_watchlist(watchlist_id, claims.user_id).await?;

    Ok(Json(watchlist.into()))
}

/// PUT /api/alerts/watchlist/:id
/// Update a watchlist
pub async fn update_watchlist(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(watchlist_id): Path<Uuid>,
    Json(request): Json<UpdateWatchlistRequest>,
) -> Result<Json<WatchlistResponse>> {
    tracing::info!(
        "Updating watchlist {} for user: {}",
        watchlist_id,
        claims.user_id
    );

    let service = NotificationService::new(config.database_pool.clone());
    let watchlist = service.update_watchlist(watchlist_id, claims.user_id, request).await?;

    Ok(Json(watchlist.into()))
}

/// DELETE /api/alerts/watchlist/:id
/// Delete a watchlist
pub async fn delete_watchlist(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(watchlist_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    tracing::info!(
        "Deleting watchlist {} for user: {}",
        watchlist_id,
        claims.user_id
    );

    let service = NotificationService::new(config.database_pool.clone());
    service.delete_watchlist(watchlist_id, claims.user_id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Watchlist deleted"
    })))
}

/// GET /api/alerts/watchlist/:id/matches
/// Get matching marketplace items for a watchlist
pub async fn get_watchlist_matches(
    State(config): State<AppConfig>,
    Extension(claims): Extension<Claims>,
    Path(watchlist_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    use crate::models::inventory::InventoryResponse;

    let service = NotificationService::new(config.database_pool.clone());
    let watchlist = service.get_watchlist(watchlist_id, claims.user_id).await?;

    // Extract search criteria
    let criteria = &watchlist.search_criteria;
    let search_term = criteria.get("search_term").and_then(|v| v.as_str()).map(|s| format!("%{}%", s));

    // Query marketplace for matches
    let matches = sqlx::query!(
        r#"
        SELECT
            i.id,
            i.pharmaceutical_id,
            i.batch_number,
            i.quantity,
            i.unit_price::TEXT as unit_price,
            i.expiry_date,
            i.storage_location,
            i.status,
            i.created_at,
            i.updated_at,
            i.user_id,
            u.company_name
        FROM inventory i
        JOIN users u ON i.user_id = u.id
        JOIN pharmaceuticals p ON i.pharmaceutical_id = p.id
        WHERE i.status = 'available'
          AND i.user_id != $1
          AND ($2::TEXT IS NULL OR
               p.brand_name ILIKE $2 OR
               p.generic_name ILIKE $2 OR
               p.manufacturer ILIKE $2)
        ORDER BY i.created_at DESC
        LIMIT 50
        "#,
        claims.user_id,
        search_term
    )
    .fetch_all(&config.database_pool)
    .await?;

    let result: Vec<serde_json::Value> = matches.iter().map(|m| {
        serde_json::json!({
            "id": m.id,
            "pharmaceutical_id": m.pharmaceutical_id,
            "batch_number": m.batch_number,
            "quantity": m.quantity,
            "unit_price": m.unit_price,
            "expiry_date": m.expiry_date,
            "storage_location": m.storage_location,
            "status": m.status,
            "seller_company_name": m.company_name,
        })
    }).collect();

    Ok(Json(serde_json::json!({
        "matches": result,
        "count": result.len()
    })))
}

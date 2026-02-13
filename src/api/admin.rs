//! Admin API endpoints (standalone mode only)
//!
//! - GET /api/admin/users — list all users
//! - DELETE /api/admin/users/:user_id — delete a user
//! - POST /api/admin/users/:user_id/promote — promote user to admin

use crate::auth::middleware::AdminClaims;
use crate::error::AppError;
use axum::{extract::State, Json};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct AdminUserInfo {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub is_admin: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/admin/users — list all users
pub async fn list_users(
    State(pool): State<PgPool>,
    _admin: AdminClaims,
) -> Result<Json<Vec<AdminUserInfo>>, AppError> {
    let users = sqlx::query_as::<_, (Uuid, String, String, bool, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, email, name, is_admin, created_at FROM users ORDER BY created_at ASC",
    )
    .fetch_all(&pool)
    .await?;

    let users: Vec<AdminUserInfo> = users
        .into_iter()
        .map(|(id, email, name, is_admin, created_at)| AdminUserInfo {
            id,
            email,
            name,
            is_admin,
            created_at,
        })
        .collect();

    Ok(Json(users))
}

/// DELETE /api/admin/users/:user_id — delete a user
pub async fn delete_user(
    State(pool): State<PgPool>,
    _admin: AdminClaims,
    axum::extract::Path(user_id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Prevent self-deletion
    let admin_id: Uuid = _admin
        .0
        .user_id
        .parse()
        .map_err(|_| AppError::Internal("Invalid admin user_id".to_string()))?;

    if admin_id == user_id {
        return Err(AppError::Forbidden(
            "Cannot delete your own account".to_string(),
        ));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("user", &user_id.to_string()));
    }

    Ok(Json(serde_json::json!({"message": "User deleted"})))
}

/// POST /api/admin/users/:user_id/promote — promote user to admin
pub async fn promote_user(
    State(pool): State<PgPool>,
    _admin: AdminClaims,
    axum::extract::Path(user_id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = sqlx::query("UPDATE users SET is_admin = true WHERE id = $1")
        .bind(user_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("user", &user_id.to_string()));
    }

    Ok(Json(
        serde_json::json!({"message": "User promoted to admin"}),
    ))
}

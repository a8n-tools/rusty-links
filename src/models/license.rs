//! License model and database operations

use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// License entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct License {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub full_name: String,
    pub created_at: DateTime<Utc>,
}

impl License {
    /// Get all available licenses (global + user's custom)
    pub async fn get_all_available(pool: &PgPool, user_id: Uuid) -> Result<Vec<License>, AppError> {
        let licenses = sqlx::query_as::<_, License>(
            r#"
            SELECT * FROM licenses
            WHERE user_id IS NULL OR user_id = $1
            ORDER BY name
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(licenses)
    }

    /// Create a user-specific license
    pub async fn create(pool: &PgPool, user_id: Uuid, name: &str, full_name: &str) -> Result<License, AppError> {
        let license = sqlx::query_as::<_, License>(
            r#"
            INSERT INTO licenses (user_id, name, full_name)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(name)
        .bind(full_name)
        .fetch_one(pool)
        .await?;

        tracing::info!(license_id = %license.id, name = %license.name, "License created");

        Ok(license)
    }

    /// Delete a user-created license (cannot delete global licenses)
    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            "DELETE FROM licenses WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("license", &id.to_string()));
        }

        tracing::info!(license_id = %id, "License deleted");

        Ok(())
    }
}

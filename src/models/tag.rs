//! Tag model and database operations

use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Tag entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new tag
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTag {
    pub name: String,
}

impl Tag {
    /// Create a new tag
    pub async fn create(pool: &PgPool, user_id: Uuid, name: &str) -> Result<Tag, AppError> {
        let tag = sqlx::query_as::<_, Tag>(
            r#"
            INSERT INTO tags (user_id, name)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(name)
        .fetch_one(pool)
        .await?;

        tracing::info!(tag_id = %tag.id, name = %tag.name, "Tag created");

        Ok(tag)
    }

    /// Get a tag by ID
    pub async fn get_by_id(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Tag, AppError> {
        sqlx::query_as::<_, Tag>("SELECT * FROM tags WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::not_found("tag", &id.to_string()))
    }

    /// Get all tags for a user
    pub async fn get_all_by_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Tag>, AppError> {
        let tags = sqlx::query_as::<_, Tag>(
            "SELECT * FROM tags WHERE user_id = $1 ORDER BY name",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(tags)
    }

    /// Delete a tag
    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM tags WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("tag", &id.to_string()));
        }

        tracing::info!(tag_id = %id, "Tag deleted");

        Ok(())
    }
}

//! Language model and database operations

use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Language entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Language {
    pub id: Uuid,
    pub name: String,
    pub user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

impl Language {
    /// Get all available languages (global + user's custom)
    pub async fn get_all_available(pool: &PgPool, user_id: Uuid) -> Result<Vec<Language>, AppError> {
        let languages = sqlx::query_as::<_, Language>(
            r#"
            SELECT * FROM languages
            WHERE user_id IS NULL OR user_id = $1
            ORDER BY name
            "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(languages)
    }

    /// Create a user-specific language
    pub async fn create(pool: &PgPool, user_id: Uuid, name: &str) -> Result<Language, AppError> {
        let language = sqlx::query_as::<_, Language>(
            r#"
            INSERT INTO languages (user_id, name)
            VALUES ($1, $2)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(name)
        .fetch_one(pool)
        .await?;

        tracing::info!(language_id = %language.id, name = %language.name, "Language created");

        Ok(language)
    }

    /// Delete a user-created language (cannot delete global languages)
    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query(
            "DELETE FROM languages WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::not_found("language", &id.to_string()));
        }

        tracing::info!(language_id = %id, "Language deleted");

        Ok(())
    }
}

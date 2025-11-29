//! Category model and database operations
//!
//! This module handles category management with hierarchical support (3 levels max).

use crate::error::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Category entity
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub depth: i32,
    pub sort_order: Option<i32>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new category
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCategory {
    pub name: String,
    pub parent_id: Option<Uuid>,
}

/// Category with its children for hierarchical display
#[derive(Debug, Clone, Serialize)]
pub struct CategoryWithChildren {
    #[serde(flatten)]
    pub category: Category,
    pub children: Vec<CategoryWithChildren>,
}

impl Category {
    /// Create a new category
    pub async fn create(
        pool: &PgPool,
        user_id: Uuid,
        create: CreateCategory,
    ) -> Result<Category, AppError> {
        let (depth, parent_id) = if let Some(pid) = create.parent_id {
            // Validate parent exists and belongs to user
            let parent = sqlx::query_as::<_, Category>(
                "SELECT * FROM categories WHERE id = $1 AND user_id = $2"
            )
            .bind(pid)
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::not_found("category", &pid.to_string()))?;

            let new_depth = parent.depth + 1;
            if new_depth > 2 {
                return Err(AppError::validation(
                    "parent_id",
                    "Maximum category depth (3 levels) exceeded",
                ));
            }

            (new_depth, Some(pid))
        } else {
            (0, None)
        };

        let category = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (user_id, name, parent_id, depth)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&create.name)
        .bind(parent_id)
        .bind(depth)
        .fetch_one(pool)
        .await?;

        tracing::info!(category_id = %category.id, name = %category.name, "Category created");

        Ok(category)
    }

    /// Get a category by ID
    pub async fn get_by_id(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<Category, AppError> {
        sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE id = $1 AND user_id = $2"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found("category", &id.to_string()))
    }

    /// Get all categories for a user (flat list)
    pub async fn get_all_by_user(pool: &PgPool, user_id: Uuid) -> Result<Vec<Category>, AppError> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE user_id = $1 ORDER BY depth, name"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(categories)
    }

    /// Get category by name, or create it if it doesn't exist
    pub async fn get_or_create_by_name(
        pool: &PgPool,
        user_id: Uuid,
        name: &str,
    ) -> Result<Category, AppError> {
        // Try to find existing category with this name
        let existing = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE user_id = $1 AND name = $2"
        )
        .bind(user_id)
        .bind(name)
        .fetch_optional(pool)
        .await?;

        if let Some(category) = existing {
            return Ok(category);
        }

        // Create new category (top-level, no parent)
        let category = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (user_id, name, parent_id, depth)
            VALUES ($1, $2, NULL, 0)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(name)
        .fetch_one(pool)
        .await?;

        tracing::info!(category_id = %category.id, name = %category.name, "Category created via import");

        Ok(category)
    }

    /// Get categories as a hierarchical tree
    pub async fn get_tree_by_user(
        pool: &PgPool,
        user_id: Uuid,
    ) -> Result<Vec<CategoryWithChildren>, AppError> {
        let categories = Self::get_all_by_user(pool, user_id).await?;
        Ok(build_category_tree(categories))
    }

    /// Update category name
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        user_id: Uuid,
        name: &str,
    ) -> Result<Category, AppError> {
        let category = sqlx::query_as::<_, Category>(
            r#"
            UPDATE categories
            SET name = $3
            WHERE id = $1 AND user_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(name)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::not_found("category", &id.to_string()))?;

        tracing::info!(category_id = %id, "Category updated");

        Ok(category)
    }

    /// Delete a category and its children (cascade)
    pub async fn delete(pool: &PgPool, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        // First verify the category exists and belongs to user
        let _ = Self::get_by_id(pool, id, user_id).await?;

        // Delete children first (depth 2, then 1), then the category itself
        // Using recursive delete via depth ordering
        sqlx::query(
            r#"
            DELETE FROM categories
            WHERE user_id = $1 AND (id = $2 OR parent_id = $2 OR parent_id IN (
                SELECT id FROM categories WHERE parent_id = $2 AND user_id = $1
            ))
            "#,
        )
        .bind(user_id)
        .bind(id)
        .execute(pool)
        .await?;

        tracing::info!(category_id = %id, "Category and children deleted");

        Ok(())
    }
}

/// Build a hierarchical tree from a flat list of categories
fn build_category_tree(categories: Vec<Category>) -> Vec<CategoryWithChildren> {
    let mut root_categories: Vec<CategoryWithChildren> = Vec::new();

    // Get root categories (depth 0, no parent)
    for cat in categories.iter().filter(|c| c.parent_id.is_none()) {
        let children = build_children(cat.id, &categories);
        root_categories.push(CategoryWithChildren {
            category: cat.clone(),
            children,
        });
    }

    root_categories
}

/// Recursively build children for a category
fn build_children(parent_id: Uuid, all_categories: &[Category]) -> Vec<CategoryWithChildren> {
    all_categories
        .iter()
        .filter(|c| c.parent_id == Some(parent_id))
        .map(|cat| {
            let children = build_children(cat.id, all_categories);
            CategoryWithChildren {
                category: cat.clone(),
                children,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_category_tree() {
        let now = Utc::now();
        let user_id = Uuid::new_v4();
        let root_id = Uuid::new_v4();
        let child_id = Uuid::new_v4();

        let categories = vec![
            Category {
                id: root_id,
                user_id,
                name: "Root".to_string(),
                parent_id: None,
                depth: 0,
                sort_order: None,
                created_at: now,
            },
            Category {
                id: child_id,
                user_id,
                name: "Child".to_string(),
                parent_id: Some(root_id),
                depth: 1,
                sort_order: None,
                created_at: now,
            },
        ];

        let tree = build_category_tree(categories);
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].category.name, "Root");
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].category.name, "Child");
    }
}

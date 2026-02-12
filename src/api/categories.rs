//! Category management API endpoints

use crate::auth::middleware::AuthenticatedUser;
use crate::error::AppError;
use crate::models::{Category, CategoryWithChildren, CreateCategory};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct CategoryResponse {
    id: Uuid,
    name: String,
    parent_id: Option<Uuid>,
    depth: i32,
    link_count: i64,
}

/// POST /api/categories
async fn create_category(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
    Json(request): Json<CreateCategory>,
) -> Result<impl IntoResponse, AppError> {
    let category = Category::create(&pool, auth.user_id, request).await?;
    let response = CategoryResponse {
        id: category.id,
        name: category.name,
        parent_id: category.parent_id,
        depth: category.depth,
        link_count: 0,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/categories
async fn list_categories(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let categories = sqlx::query_as::<_, (Uuid, String, Option<Uuid>, i32, i64)>(
        r#"
        SELECT c.id, c.name, c.parent_id, c.depth, COUNT(lc.link_id) as link_count
        FROM categories c
        LEFT JOIN link_categories lc ON c.id = lc.category_id
        LEFT JOIN links l ON lc.link_id = l.id AND l.user_id = $1
        WHERE c.user_id = $1
        GROUP BY c.id, c.name, c.parent_id, c.depth
        ORDER BY c.depth, c.name
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<CategoryResponse> = categories
        .into_iter()
        .map(
            |(id, name, parent_id, depth, link_count)| CategoryResponse {
                id,
                name,
                parent_id,
                depth,
                link_count,
            },
        )
        .collect();

    Ok(Json(response))
}

/// GET /api/categories/tree
async fn get_category_tree(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
) -> Result<Json<Vec<CategoryWithChildren>>, AppError> {
    let tree = Category::get_tree_by_user(&pool, auth.user_id).await?;
    Ok(Json(tree))
}

/// GET /api/categories/:id
async fn get_category(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<Json<CategoryResponse>, AppError> {
    let result = sqlx::query_as::<_, (Uuid, String, Option<Uuid>, i32, i64)>(
        r#"
        SELECT c.id, c.name, c.parent_id, c.depth, COUNT(lc.link_id) as link_count
        FROM categories c
        LEFT JOIN link_categories lc ON c.id = lc.category_id
        LEFT JOIN links l ON lc.link_id = l.id AND l.user_id = $1
        WHERE c.id = $2 AND c.user_id = $1
        GROUP BY c.id, c.name, c.parent_id, c.depth
        "#,
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::not_found("category", &id.to_string()))?;

    let response = CategoryResponse {
        id: result.0,
        name: result.1,
        parent_id: result.2,
        depth: result.3,
        link_count: result.4,
    };

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
struct UpdateCategoryRequest {
    name: String,
}

/// PUT /api/categories/:id
async fn update_category(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    let category = Category::update(&pool, id, auth.user_id, &request.name).await?;

    let link_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(lc.link_id)
        FROM link_categories lc
        JOIN links l ON lc.link_id = l.id AND l.user_id = $1
        WHERE lc.category_id = $2
        "#,
    )
    .bind(auth.user_id)
    .bind(id)
    .fetch_one(&pool)
    .await?;

    let response = CategoryResponse {
        id: category.id,
        name: category.name,
        parent_id: category.parent_id,
        depth: category.depth,
        link_count,
    };

    Ok(Json(response))
}

/// DELETE /api/categories/:id
async fn delete_category(
    State(pool): State<PgPool>,
    auth: AuthenticatedUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    Category::delete(&pool, id, auth.user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Create the categories router
pub fn create_router() -> Router<super::AppState> {
    Router::new()
        .route("/", post(create_category).get(list_categories))
        .route("/tree", get(get_category_tree))
        .route(
            "/{id}",
            get(get_category)
                .put(update_category)
                .delete(delete_category),
        )
}

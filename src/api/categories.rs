//! Category management API endpoints

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{Category, CategoryWithChildren, CreateCategory, User};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to get authenticated user
async fn get_authenticated_user(pool: &PgPool, jar: &CookieJar) -> Result<User, AppError> {
    let session_id = get_session_from_cookies(jar).ok_or(AppError::SessionExpired)?;
    let session = get_session(pool, &session_id).await?.ok_or(AppError::SessionExpired)?;

    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, name, created_at FROM users WHERE id = $1"
    )
    .bind(session.user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

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
    jar: CookieJar,
    Json(request): Json<CreateCategory>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let category = Category::create(&pool, user.id, request).await?;
    let response = CategoryResponse {
        id: category.id,
        name: category.name,
        parent_id: category.parent_id,
        depth: category.depth,
        link_count: 0, // New category has no links yet
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/categories
async fn list_categories(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<CategoryResponse>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

    // Fetch categories with link counts
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
    .bind(user.id)
    .fetch_all(&pool)
    .await?;

    let response: Vec<CategoryResponse> = categories
        .into_iter()
        .map(|(id, name, parent_id, depth, link_count)| CategoryResponse {
            id,
            name,
            parent_id,
            depth,
            link_count,
        })
        .collect();

    Ok(Json(response))
}

/// GET /api/categories/tree
async fn get_category_tree(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<CategoryWithChildren>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let tree = Category::get_tree_by_user(&pool, user.id).await?;
    Ok(Json(tree))
}

/// GET /api/categories/:id
async fn get_category(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<Json<CategoryResponse>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;

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
    .bind(user.id)
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
    jar: CookieJar,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let category = Category::update(&pool, id, user.id, &request.name).await?;

    // Get link count for this category
    let link_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(lc.link_id)
        FROM link_categories lc
        JOIN links l ON lc.link_id = l.id AND l.user_id = $1
        WHERE lc.category_id = $2
        "#,
    )
    .bind(user.id)
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
    jar: CookieJar,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    Category::delete(&pool, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Create the categories router
pub fn create_router() -> Router<PgPool> {
    Router::new()
        .route("/", post(create_category).get(list_categories))
        .route("/tree", get(get_category_tree))
        .route("/{id}", get(get_category).put(update_category).delete(delete_category))
}

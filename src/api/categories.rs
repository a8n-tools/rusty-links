//! Category management API endpoints

use crate::auth::{get_session, get_session_from_cookies};
use crate::error::AppError;
use crate::models::{Category, CategoryWithChildren, CreateCategory, User};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Helper to get authenticated user
async fn get_authenticated_user(pool: &PgPool, jar: &CookieJar) -> Result<User, AppError> {
    let session_id = get_session_from_cookies(jar).ok_or(AppError::SessionExpired)?;
    let session = get_session(pool, &session_id).await?.ok_or(AppError::SessionExpired)?;

    sqlx::query_as::<_, User>(
        "SELECT id, email, password_hash, created_at FROM users WHERE id = $1"
    )
    .bind(session.user_id)
    .fetch_one(pool)
    .await
    .map_err(AppError::from)
}

/// POST /api/categories
async fn create_category(
    State(pool): State<PgPool>,
    jar: CookieJar,
    Json(request): Json<CreateCategory>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let category = Category::create(&pool, user.id, request).await?;
    Ok((StatusCode::CREATED, Json(category)))
}

/// GET /api/categories
async fn list_categories(
    State(pool): State<PgPool>,
    jar: CookieJar,
) -> Result<Json<Vec<Category>>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let categories = Category::get_all_by_user(&pool, user.id).await?;
    Ok(Json(categories))
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
) -> Result<Json<Category>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let category = Category::get_by_id(&pool, id, user.id).await?;
    Ok(Json(category))
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
) -> Result<Json<Category>, AppError> {
    let user = get_authenticated_user(&pool, &jar).await?;
    let category = Category::update(&pool, id, user.id, &request.name).await?;
    Ok(Json(category))
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
        .route("/:id", get(get_category).put(update_category).delete(delete_category))
}

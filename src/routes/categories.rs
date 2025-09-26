use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{group_guard::group_guard, AuthContext},
    error::AppError,
    middleware::tier::check_tier_limit,
    repos::{
        category::{Category, CategoryRepo, CreateCategoryDbPayload, UpdateCategoryDbPayload},
        subscription::SubscriptionRepo,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/groups/{group_uid}/categories", axum::routing::get(list))
        .route("/categories", axum::routing::post(create))
        .route(
            "/categories/{uid}",
            axum::routing::get(get).put(update),
        )
}

#[utoipa::path(
    get, 
    path = "/groups/{group_uid}/categories", 
    params(("group_uid" = Uuid, Path)),
    responses((status = 200, body = [Category])), 
    tag = "Categories", 
    operation_id = "listCategories", 
    security(("bearerAuth" = []))
)]
pub async fn list(
    Extension(auth): Extension<AuthContext>,
    State(state): State<AppState>,
    Path(group_uid): Path<Uuid>,
) -> Result<Json<Vec<Category>>, AppError> {
    group_guard(&auth, group_uid, &state.db_pool).await?;
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for listing categories"))?;
    let res = CategoryRepo::list_by_group(&mut tx, group_uid).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for listing categories"))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/categories/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = Category)), tag = "Categories", operation_id = "getCategory", security(("bearerAuth" = [])))]
pub async fn get(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Category>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for getting category"))?;
    let prev_category = CategoryRepo::get(&mut tx, uid).await?;
    group_guard(&auth, prev_category.group_uid, &state.db_pool).await?;
    let res = CategoryRepo::get(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for getting category"))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCategoryPayload {
    pub group_uid: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[utoipa::path(
    post,
    path = "/categories", 
    request_body = CreateCategoryPayload, 
    responses((status = 200, body = Category)), 
    tag = "Categories", 
    operation_id = "createCategory", 
    security(("bearerAuth" = [])))
]
pub async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateCategoryPayload>,
) -> Result<Json<Category>, AppError> {
    group_guard(&auth, payload.group_uid, &state.db_pool).await?;

    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for creating category"))?;

    // Get user's subscription
    let subscription = SubscriptionRepo::get_by_user(&mut tx, auth.user_uid).await?;

    // Check category limit per group
    let current_categories = CategoryRepo::count_by_group(&mut tx, payload.group_uid).await?;
    check_tier_limit(&subscription, "categories_per_group", current_categories as i32)?;

    let created = CategoryRepo::create(
        &mut tx,
        CreateCategoryDbPayload {
            group_uid: payload.group_uid,
            name: payload.name,
            description: payload.description,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for creating category"))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCategoryPayload {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[utoipa::path(put, path = "/categories/{uid}", params(("uid" = Uuid, Path)), request_body = UpdateCategoryPayload, responses((status = 200, body = Category)), tag = "Categories", operation_id = "updateCategory", security(("bearerAuth" = [])))]
pub async fn update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateCategoryPayload>,
) -> Result<Json<Category>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for updating category"))?;
    let prev_category = CategoryRepo::get(&mut tx, uid).await?;

    group_guard(&auth, prev_category.group_uid, &state.db_pool).await?;

    let updated = CategoryRepo::update(
        &mut tx,
        uid,
        UpdateCategoryDbPayload {
            name: payload.name,
            description: payload.description,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for updating category"))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/categories/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Categories", operation_id = "deleteCategory", security(("bearerAuth" = [])))]
pub async fn delete_(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>, 
    Path(uid): Path<Uuid>
) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for deleting category"))?;
    let prev_category = CategoryRepo::get(&mut tx, uid).await?;
    group_guard(&auth, prev_category.group_uid, &state.db_pool).await?;
    CategoryRepo::delete(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for deleting category"))?;
    Ok(())
}

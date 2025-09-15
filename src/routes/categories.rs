use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::AppError,
    repos::category::{Category, CategoryRepo, CreateCategoryPayload, UpdateCategoryPayload},
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route(
            "/{uid}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

#[utoipa::path(get, path = "/categories", responses((status = 200, body = [Category])), tag = "Categories", operation_id = "listCategories")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Category>>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let res = CategoryRepo::list(&mut tx).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/categories/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = Category)), tag = "Categories", operation_id = "getCategory")]
pub async fn get(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Category>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let res = CategoryRepo::get(&mut tx, uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload {
    pub group_uid: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[utoipa::path(post, path = "/categories", request_body = CreatePayload, responses((status = 200, body = Category)), tag = "Categories", operation_id = "createCategory")]
pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<Category>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let created = CategoryRepo::create(
        &mut tx,
        CreateCategoryPayload {
            group_uid: payload.group_uid,
            name: payload.name,
            description: payload.description,
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[utoipa::path(put, path = "/categories/{uid}", params(("uid" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = Category)), tag = "Categories", operation_id = "updateCategory")]
pub async fn update(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<Category>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let updated = CategoryRepo::update(
        &mut tx,
        uid,
        UpdateCategoryPayload {
            name: payload.name,
            description: payload.description,
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/categories/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Categories", operation_id = "deleteCategory")]
pub async fn delete_(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    CategoryRepo::delete(&mut tx, uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(())
}

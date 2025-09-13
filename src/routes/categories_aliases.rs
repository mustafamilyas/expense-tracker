use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{error::app::AppError, repos::category_alias::{CategoryAlias, CategoryAliasRepo, CreateCategoryAliasPayload, UpdateCategoryAliasPayload}, types::AppState};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{alias_uid}", axum::routing::get(get).put(update).delete(delete_))
}

#[utoipa::path(get, path = "/categories-aliases", responses((status = 200, body = [CategoryAlias])), tag = "Category Aliases")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<CategoryAlias>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = CategoryAliasRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/categories-aliases/{alias_uid}", params(("alias_uid" = Uuid, Path)), responses((status = 200, body = CategoryAlias)), tag = "Category Aliases")]
pub async fn get(State(state): State<AppState>, Path(alias_uid): Path<Uuid>) -> Result<Json<CategoryAlias>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = CategoryAliasRepo::get(&mut tx, alias_uid).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload { pub group_uid: Uuid, pub alias: String, pub category_uid: Uuid }

#[utoipa::path(post, path = "/categories-aliases", request_body = CreatePayload, responses((status = 200, body = CategoryAlias)), tag = "Category Aliases")]
pub async fn create(State(state): State<AppState>, Json(payload): Json<CreatePayload>) -> Result<Json<CategoryAlias>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let created = CategoryAliasRepo::create(&mut tx, CreateCategoryAliasPayload { group_uid: payload.group_uid, alias: payload.alias, category_uid: payload.category_uid }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload { pub alias: Option<String>, pub category_uid: Option<Uuid> }

#[utoipa::path(put, path = "/categories-aliases/{alias_uid}", params(("alias_uid" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = CategoryAlias)), tag = "Category Aliases")]
pub async fn update(State(state): State<AppState>, Path(alias_uid): Path<Uuid>, Json(payload): Json<UpdatePayload>) -> Result<Json<CategoryAlias>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let updated = CategoryAliasRepo::update(&mut tx, alias_uid, UpdateCategoryAliasPayload { alias: payload.alias, category_uid: payload.category_uid }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/categories-aliases/{alias_uid}", params(("alias_uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Category Aliases")]
pub async fn delete_(State(state): State<AppState>, Path(alias_uid): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    CategoryAliasRepo::delete(&mut tx, alias_uid).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(())
}

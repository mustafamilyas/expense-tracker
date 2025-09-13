use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{error::app::AppError, repos::expense_group::{ExpenseGroup, ExpenseGroupRepo, CreateExpenseGroupPayload, UpdateExpenseGroupPayload}, types::AppState};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{uid}", axum::routing::get(get).put(update).delete(delete_))
}

#[utoipa::path(get, path = "/expense-groups", responses((status = 200, body = [ExpenseGroup])), tag = "Expense Groups")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ExpenseGroup>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = ExpenseGroupRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/expense-groups/{uid}", params(("uid" = Uuid, Path, description = "Group uid")), responses((status = 200, body = ExpenseGroup)), tag = "Expense Groups")]
pub async fn get(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<ExpenseGroup>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = ExpenseGroupRepo::get(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload { pub name: String, pub owner: Uuid }

#[utoipa::path(post, path = "/expense-groups", request_body = CreatePayload, responses((status = 200, body = ExpenseGroup)), tag = "Expense Groups")]
pub async fn create(State(state): State<AppState>, Json(payload): Json<CreatePayload>) -> Result<Json<ExpenseGroup>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let created = ExpenseGroupRepo::create(&mut tx, CreateExpenseGroupPayload { name: payload.name, owner: payload.owner }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload { pub name: Option<String> }

#[utoipa::path(put, path = "/expense-groups/{uid}", params(("uid" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = ExpenseGroup)), tag = "Expense Groups")]
pub async fn update(State(state): State<AppState>, Path(uid): Path<Uuid>, Json(payload): Json<UpdatePayload>) -> Result<Json<ExpenseGroup>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let updated = ExpenseGroupRepo::update(&mut tx, uid, UpdateExpenseGroupPayload { name: payload.name }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/expense-groups/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Expense Groups")]
pub async fn delete_(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    ExpenseGroupRepo::delete(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(())
}

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
    Ok(Json(ExpenseGroupRepo::list(&state.db_pool).await?))
}

#[utoipa::path(get, path = "/expense-groups/{uid}", params(("uid" = Uuid, Path, description = "Group uid")), responses((status = 200, body = ExpenseGroup)), tag = "Expense Groups")]
pub async fn get(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<ExpenseGroup>, AppError> {
    Ok(Json(ExpenseGroupRepo::get(&state.db_pool, uid).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload { pub name: String, pub owner: Uuid }

#[utoipa::path(post, path = "/expense-groups", request_body = CreatePayload, responses((status = 200, body = ExpenseGroup)), tag = "Expense Groups")]
pub async fn create(State(state): State<AppState>, Json(payload): Json<CreatePayload>) -> Result<Json<ExpenseGroup>, AppError> {
    let created = ExpenseGroupRepo::create(&state.db_pool, CreateExpenseGroupPayload { name: payload.name, owner: payload.owner }).await?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload { pub name: Option<String> }

#[utoipa::path(put, path = "/expense-groups/{uid}", params(("uid" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = ExpenseGroup)), tag = "Expense Groups")]
pub async fn update(State(state): State<AppState>, Path(uid): Path<Uuid>, Json(payload): Json<UpdatePayload>) -> Result<Json<ExpenseGroup>, AppError> {
    let updated = ExpenseGroupRepo::update(&state.db_pool, uid, UpdateExpenseGroupPayload { name: payload.name }).await?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/expense-groups/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Expense Groups")]
pub async fn delete_(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<(), AppError> {
    ExpenseGroupRepo::delete(&state.db_pool, uid).await?;
    Ok(())
}

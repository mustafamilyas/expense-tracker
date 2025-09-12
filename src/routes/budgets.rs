use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{error::app::AppError, repos::budget::{Budget, BudgetRepo, CreateBudgetPayload, UpdateBudgetPayload}, types::AppState};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{uid}", axum::routing::get(get).put(update).delete(delete_))
}

#[utoipa::path(get, path = "/budgets", responses((status = 200, body = [Budget])), tag = "Budgets")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Budget>>, AppError> {
    Ok(Json(BudgetRepo::list(&state.db_pool).await?))
}

#[utoipa::path(get, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = Budget)), tag = "Budgets")]
pub async fn get(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<Budget>, AppError> {
    Ok(Json(BudgetRepo::get(&state.db_pool, uid).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload { pub group_uid: Uuid, pub category_uid: Uuid, pub amount: f64, pub period_year: Option<i32>, pub period_month: Option<i32> }

#[utoipa::path(post, path = "/budgets", request_body = CreatePayload, responses((status = 200, body = Budget)), tag = "Budgets")]
pub async fn create(State(state): State<AppState>, Json(payload): Json<CreatePayload>) -> Result<Json<Budget>, AppError> {
    let created = BudgetRepo::create(&state.db_pool, CreateBudgetPayload {
        group_uid: payload.group_uid,
        category_uid: payload.category_uid,
        amount: payload.amount,
        period_year: payload.period_year,
        period_month: payload.period_month,
    }).await?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload { pub amount: Option<f64>, pub period_year: Option<i32>, pub period_month: Option<i32> }

#[utoipa::path(put, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = Budget)), tag = "Budgets")]
pub async fn update(State(state): State<AppState>, Path(uid): Path<Uuid>, Json(payload): Json<UpdatePayload>) -> Result<Json<Budget>, AppError> {
    let updated = BudgetRepo::update(&state.db_pool, uid, UpdateBudgetPayload { amount: payload.amount, period_year: payload.period_year, period_month: payload.period_month }).await?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Budgets")]
pub async fn delete_(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<(), AppError> {
    BudgetRepo::delete(&state.db_pool, uid).await?;
    Ok(())
}

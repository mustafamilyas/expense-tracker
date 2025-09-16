use axum::{
    Json,
    extract::{Path, State, Extension},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::AppError,
    repos::budget::{Budget, BudgetRepo, CreateBudgetPayload, UpdateBudgetPayload},
    types::AppState,
    auth::{AuthContext, AuthSource},
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route(
            "/{uid}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

#[utoipa::path(get, path = "/budgets", responses((status = 200, body = [Budget])), tag = "Budgets", operation_id = "listBudgets", security(("bearerAuth" = [])))]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Budget>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = BudgetRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = Budget)), tag = "Budgets", operation_id = "getBudget", security(("bearerAuth" = [])))]
pub async fn get(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<Budget>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = BudgetRepo::get(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload {
    pub group_uid: Uuid,
    pub category_uid: Uuid,
    pub amount: f64,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

#[utoipa::path(post, path = "/budgets", request_body = CreatePayload, responses((status = 200, body = Budget)), tag = "Budgets", operation_id = "createBudget", security(("bearerAuth" = [])))]
pub async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreatePayload>,
) -> Result<Json<Budget>, AppError> {
    if matches!(auth.source, AuthSource::Chat) && auth.group_uid != Some(payload.group_uid) {
        return Err(AppError::Unauthorized("Group scope mismatch".into()));
    }
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let created = BudgetRepo::create(
        &mut tx,
        CreateBudgetPayload {
            group_uid: payload.group_uid,
            category_uid: payload.category_uid,
            amount: payload.amount,
            period_year: payload.period_year,
            period_month: payload.period_month,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload {
    pub amount: Option<f64>,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

#[utoipa::path(put, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = Budget)), tag = "Budgets", operation_id = "updateBudget", security(("bearerAuth" = [])))]
pub async fn update(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdatePayload>,
) -> Result<Json<Budget>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let updated = BudgetRepo::update(
        &mut tx,
        uid,
        UpdateBudgetPayload {
            amount: payload.amount,
            period_year: payload.period_year,
            period_month: payload.period_month,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Budgets", operation_id = "deleteBudget", security(("bearerAuth" = [])))]
pub async fn delete_(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    BudgetRepo::delete(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(())
}

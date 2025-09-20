use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, group_guard::group_guard},
    error::AppError,
    repos::budget::{Budget, BudgetRepo, CreateBudgetDbPayload, UpdateBudgetDbPayload},
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/budgets", axum::routing::post(create))
        .route("/budgets/group/{group_uid}", axum::routing::get(list))
        .route(
            "/budgets/{uid}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

#[utoipa::path(get, path = "/budgets/group/{group_uid}", params(("group_uid" = Uuid, Path)), responses((status = 200, body = [Budget])), tag = "Budgets", operation_id = "listBudgets", security(("bearerAuth" = [])))]
pub async fn list(
    State(state): State<AppState>,
    Path(group_uid): Path<Uuid>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Vec<Budget>>, AppError> {
    group_guard(&auth, group_uid, &state.db_pool).await?;
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = BudgetRepo::list_by_group(&mut tx, group_uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = Budget)), tag = "Budgets", operation_id = "getBudget", security(("bearerAuth" = [])))]
pub async fn get(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Budget>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = BudgetRepo::get(&mut tx, uid).await?;
    group_guard(&auth, res.group_uid, &state.db_pool).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateBudgetPayload {
    pub group_uid: Uuid,
    pub category_uid: Uuid,
    pub amount: f64,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

#[utoipa::path(post, path = "/budgets", request_body = CreateBudgetPayload, responses((status = 200, body = Budget)), tag = "Budgets", operation_id = "createBudget", security(("bearerAuth" = [])))]
pub async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateBudgetPayload>,
) -> Result<Json<Budget>, AppError> {
    group_guard(&auth, payload.group_uid, &state.db_pool).await?;
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let created = BudgetRepo::create(
        &mut tx,
        CreateBudgetDbPayload {
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
pub struct UpdateBudgetPayload {
    pub amount: Option<f64>,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

#[utoipa::path(put, path = "/budgets/{uid}", params(("uid" = Uuid, Path)), request_body = UpdateBudgetPayload, responses((status = 200, body = Budget)), tag = "Budgets", operation_id = "updateBudget", security(("bearerAuth" = [])))]
pub async fn update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateBudgetPayload>,
) -> Result<Json<Budget>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let prev_rec = BudgetRepo::get(&mut tx, uid).await?;
    group_guard(&auth, prev_rec.group_uid, &state.db_pool).await?;
    let updated = BudgetRepo::update(
        &mut tx,
        uid,
        UpdateBudgetDbPayload {
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
pub async fn delete_(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Extension(auth): Extension<AuthContext>,
) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let budget = BudgetRepo::get(&mut tx, uid).await?;
    group_guard(&auth, budget.group_uid, &state.db_pool).await?;
    BudgetRepo::delete(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(())
}

use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    error::AppError,
    repos::expense_group::{
        CreateExpenseGroupPayload, ExpenseGroup, ExpenseGroupRepo, UpdateExpenseGroupPayload,
    },
    types::{AppState, DeleteResponse},
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/expense-groups", axum::routing::get(list).post(create))
        .route(
            "/expense-groups/{uid}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

#[utoipa::path(
    get, 
    path = "/expense-groups", 
    responses((status = 200, body = [ExpenseGroup])), 
    tag = "Expense Groups",
    operation_id = "listExpenseGroups"
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ExpenseGroup>>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let res = ExpenseGroupRepo::list(&mut tx).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[utoipa::path(
    get, 
    path = "/expense-groups/{uid}", 
    params(("uid" = Uuid, Path, description = "Group uid")), 
    responses((status = 200, body = ExpenseGroup)), 
    tag = "Expense Groups",
    operation_id = "getExpenseGroup"
)]
pub async fn get(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<ExpenseGroup>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let res = ExpenseGroupRepo::get(&mut tx, uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

// TODO: infer owner from auth context
#[utoipa::path(
    post, 
    path = "/expense-groups", 
    request_body = CreateExpenseGroupPayload, 
    responses((status = 200, body = ExpenseGroup)), 
    tag = "Expense Groups",
    operation_id = "createExpenseGroup"
)]
pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateExpenseGroupPayload>,
) -> Result<Json<ExpenseGroup>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let created = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupPayload {
            name: payload.name,
            owner: payload.owner,
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(created))
}

#[utoipa::path(
    put, 
    path = "/expense-groups/{uid}", 
    params(("uid" = Uuid, Path)), 
    request_body = UpdateExpenseGroupPayload, 
    responses((status = 200, body = ExpenseGroup)), 
    tag = "Expense Groups",
    operation_id = "updateExpenseGroup"
)]
pub async fn update(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateExpenseGroupPayload>,
) -> Result<Json<ExpenseGroup>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let updated = ExpenseGroupRepo::update(
        &mut tx,
        uid,
        UpdateExpenseGroupPayload { name: payload.name },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(updated))
}


// TODO: change into soft delete
// should we fail if there are expenses in the group?
#[utoipa::path(
    delete, 
    path = "/expense-groups/{uid}", 
    params(("uid" = Uuid, Path)), 
    responses((status = 200, description = "Deleted", body = DeleteResponse)), 
    tag = "Expense Groups",
    operation_id = "deleteExpenseGroup"
)]
pub async fn delete_(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<DeleteResponse>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    ExpenseGroupRepo::delete(&mut tx, uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from(e))?;
    Ok(Json(DeleteResponse {
        success: true,
    }))
}

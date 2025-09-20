use axum::{
    extract::{Path, State}, Extension, Json
};
use uuid::Uuid;

use crate::{
    auth::{ group_guard::group_guard, AuthContext}, error::AppError, repos::expense_group::{
        CreateExpenseGroupPayload, ExpenseGroup, ExpenseGroupRepo, UpdateExpenseGroupPayload,
    }, types::{AppState, DeleteResponse}
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/expense-groups", axum::routing::get(list).post(create))
        .route(
            "/expense-groups/{uid}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

// TODO: filter this to admin
#[utoipa::path(
    get, 
    path = "/expense-groups", 
    responses((status = 200, body = [ExpenseGroup])), 
    tag = "Expense Groups",
    operation_id = "listExpenseGroups",
    security(("bearerAuth" = []))
)]
pub async fn list(State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>
) -> Result<Json<Vec<ExpenseGroup>>, AppError> {

    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from(e))?;
    let res = ExpenseGroupRepo::get_all_by_owner(&mut tx, auth.user_uid).await?;
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
    operation_id = "getExpenseGroup",
    security(("bearerAuth" = []))
)]
pub async fn get(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<ExpenseGroup>, AppError> {
    group_guard(&auth, uid, &state.db_pool).await?;
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
    operation_id = "createExpenseGroup",
    security(("bearerAuth" = []))
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
    operation_id = "updateExpenseGroup",
    security(("bearerAuth" = []))
)]
pub async fn update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateExpenseGroupPayload>,
) -> Result<Json<ExpenseGroup>, AppError> {
    group_guard(&auth, uid, &state.db_pool).await?;
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
    operation_id = "deleteExpenseGroup",
    security(("bearerAuth" = []))
)]
pub async fn delete_(
    State(state): State<AppState>, 
    Path(uid): Path<Uuid>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<DeleteResponse>, AppError> {
    group_guard(&auth, uid, &state.db_pool).await?;
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

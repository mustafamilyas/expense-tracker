use axum::{
    extract::{Path, State}, Extension, Json
};
use serde::Deserialize;
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{ group_guard::group_guard, AuthContext}, error::AppError,
    middleware::tier::check_tier_limit,
    repos::{
        expense_group::{
         CreateExpenseGroupDbPayload, ExpenseGroup, ExpenseGroupRepo, UpdateExpenseGroupDbPayload
        },
        subscription::SubscriptionRepo,
    },
    types::{AppState, DeleteResponse}
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/expense-groups", axum::routing::get(list).post(create))
        .route(
            "/expense-groups/{uid}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

/**
 * Get all expense groups for the authenticated user
 */
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
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for listing expense groups"))?;
    let res = ExpenseGroupRepo::get_all_by_owner(&mut tx, auth.user_uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for listing expense groups"))?;
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
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for getting expense group"))?;
    let res = ExpenseGroupRepo::get(&mut tx, uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for getting expense group"))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateExpenseGroupPayload {
    pub name: String,
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
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateExpenseGroupPayload>,
) -> Result<Json<ExpenseGroup>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for creating expense group"))?;

    // Get user's subscription
    let subscription = SubscriptionRepo::get_by_user(&mut tx, auth.user_uid).await?;

    // Check group limit
    let current_groups = ExpenseGroupRepo::count_by_owner(&mut tx, auth.user_uid).await?;
    check_tier_limit(&subscription, "groups", current_groups as i32)?;

    let created = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupDbPayload {
            name: payload.name,
            owner: auth.user_uid, // Use authenticated user as owner
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for creating expense group"))?;
    Ok(Json(created))
}

#[utoipa::path(
    put, 
    path = "/expense-groups/{uid}", 
    params(("uid" = Uuid, Path)), 
    request_body = UpdateExpenseGroupDbPayload, 
    responses((status = 200, body = ExpenseGroup)), 
    tag = "Expense Groups",
    operation_id = "updateExpenseGroup",
    security(("bearerAuth" = []))
)]
pub async fn update(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateExpenseGroupDbPayload>,
) -> Result<Json<ExpenseGroup>, AppError> {
    group_guard(&auth, uid, &state.db_pool).await?;
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for updating expense group"))?;
    let updated = ExpenseGroupRepo::update(
        &mut tx,
        uid,
        UpdateExpenseGroupDbPayload { name: payload.name },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for updating expense group"))?;
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
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for deleting expense group"))?;
    ExpenseGroupRepo::delete(&mut tx, uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for deleting expense group"))?;
    Ok(Json(DeleteResponse {
        success: true,
    }))
}

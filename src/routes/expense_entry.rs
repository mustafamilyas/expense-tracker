use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, AuthSource},
    error::AppError,
    repos::expense_entry::{
        CreateExpenseEntryPayload as CreateDbPayload, ExpenseEntry, ExpenseEntryRepo,
        UpdateExpenseEntryPayload as UpdateDbPayload,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/",
            axum::routing::get(list_expense_entries).post(create_expense_entry),
        )
        .route(
            "/{uid}",
            axum::routing::get(get_expense_entry)
                .put(update_expense_entry)
                .delete(delete_expense_entry),
        )
}

#[utoipa::path(get, path = "/expense-entries", responses((status = 200, body = [ExpenseEntry])), tag = "Expense Entries", operation_id = "listExpenseEntries", security(("bearerAuth" = [])))]
pub async fn list_expense_entries(
    State(state): State<AppState>,
) -> Result<Json<Vec<ExpenseEntry>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = ExpenseEntryRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateExpenseEntryPayload {
    pub price: f64,
    pub product: String,
    pub group_uid: Uuid,
    pub category_uid: Uuid,
}

#[utoipa::path(post, path = "/expense-entries", request_body = CreateExpenseEntryPayload, responses((status = 200, body = ExpenseEntry)), tag = "Expense Entries", operation_id = "createExpenseEntry", security(("bearerAuth" = [])))]
pub async fn create_expense_entry(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateExpenseEntryPayload>,
) -> Result<Json<ExpenseEntry>, AppError> {
    if matches!(auth.source, AuthSource::Chat) && auth.group_uid != Some(payload.group_uid) {
        return Err(AppError::Unauthorized("Group scope mismatch".into()));
    }
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let created = ExpenseEntryRepo::create_expense_entry(
        &mut tx,
        CreateDbPayload {
            price: payload.price,
            product: payload.product,
            group_uid: payload.group_uid,
            category_uid: payload.category_uid,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(created))
}

#[utoipa::path(get, path = "/expense-entries/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = ExpenseEntry)), tag = "Expense Entries", operation_id = "getExpenseEntry", security(("bearerAuth" = [])))]
pub async fn get_expense_entry(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<ExpenseEntry>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let rec = ExpenseEntryRepo::get(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(rec))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateExpenseEntryPayload {
    pub price: Option<f64>,
    pub product: Option<String>,
    pub category_uid: Option<Uuid>,
}

#[utoipa::path(put, path = "/expense-entries/{uid}", params(("uid" = Uuid, Path)), request_body = UpdateExpenseEntryPayload, responses((status = 200, body = ExpenseEntry)), tag = "Expense Entries", operation_id = "updateExpenseEntry", security(("bearerAuth" = [])))]
pub async fn update_expense_entry(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateExpenseEntryPayload>,
) -> Result<Json<ExpenseEntry>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let updated = ExpenseEntryRepo::update(
        &mut tx,
        uid,
        UpdateDbPayload {
            price: payload.price,
            product: payload.product,
            category_uid: payload.category_uid,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/expense-entries/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Expense Entries", operation_id = "deleteExpenseEntry", security(("bearerAuth" = [])))]
pub async fn delete_expense_entry(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    ExpenseEntryRepo::delete(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(())
}

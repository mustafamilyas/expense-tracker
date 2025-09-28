use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Deserialize;
use serde_json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, group_guard::group_guard},
    error::AppError,
    middleware::tier::check_tier_limit,
    repos::{
        expense_entry::{
            CreateExpenseEntryDbPayload, ExpenseEntry, ExpenseEntryRepo,
            UpdateExpenseEntryDbPayload,
        },
        subscription::SubscriptionRepo,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/expense-entries",
            axum::routing::post(create_expense_entry),
        )
        .route(
            "/groups/{group_uid}/expense-entries",
            axum::routing::get(list_expense_entries),
        )
        .route(
            "/{uid}",
            axum::routing::get(get_expense_entry)
                .put(update_expense_entry)
                .delete(delete_expense_entry),
        )
}

#[utoipa::path(get, path = "/groups/{group_uid}/expense-entries", responses((status = 200, body = [ExpenseEntry])), tag = "Expense Entries", operation_id = "listExpenseEntries", security(("bearerAuth" = [])))]
pub async fn list_expense_entries(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(group_uid): Path<Uuid>,
) -> Result<Json<Vec<ExpenseEntry>>, AppError> {
    group_guard(&auth, group_uid, &state.db_pool).await?;
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::from_sqlx_error(e, "beginning transaction for listing expense entries")
    })?;
    let res = ExpenseEntryRepo::list_by_group(&mut tx, group_uid).await?;
    tx.commit().await.map_err(|e| {
        AppError::from_sqlx_error(e, "committing transaction for listing expense entries")
    })?;
    Ok(Json(res))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateExpenseEntryPayload {
    pub price: f64,
    pub product: String,
    pub group_uid: Uuid,
    pub category_uid: Option<Uuid>,
}

#[utoipa::path(post, path = "/expense-entries", request_body = CreateExpenseEntryPayload, responses((status = 200, body = serde_json::Value)), tag = "Expense Entries", operation_id = "createExpenseEntry", security(("bearerAuth" = [])))]
pub async fn create_expense_entry(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateExpenseEntryPayload>,
) -> Result<Json<serde_json::Value>, AppError> {
    group_guard(&auth, payload.group_uid, &state.db_pool).await?;
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::from_sqlx_error(e, "beginning transaction for creating expense entry")
    })?;

    // Get user's subscription
    let subscription = SubscriptionRepo::get_by_user(&mut tx, auth.user_uid).await?;

    // Check expense limit for current month
    let usage_payload =
        crate::repos::subscription::UserUsageRepo::calculate_current_usage(&mut tx, auth.user_uid)
            .await?;
    check_tier_limit(
        &subscription,
        "expenses_per_month",
        usage_payload.total_expenses,
    )?;

    let created = ExpenseEntryRepo::create_expense_entry(
        &mut tx,
        CreateExpenseEntryDbPayload {
            price: payload.price,
            product: payload.product,
            group_uid: payload.group_uid,
            category_uid: payload.category_uid,
        },
    )
    .await?;

    // Check if near limit and include upgrade warning in response
    let limits = subscription.get_tier().limits();
    let mut response_data = serde_json::to_value(&created).unwrap();

    if limits.is_near_limit(usage_payload.total_expenses, limits.max_expenses_per_month) {
        let upgrade_message = crate::middleware::tier::get_upgrade_message(
            &subscription,
            "expenses_per_month",
            usage_payload.total_expenses as i32,
            limits.max_expenses_per_month,
        );

        if let serde_json::Value::Object(ref mut map) = response_data {
            map.insert("upgrade_warning".to_string(), upgrade_message);
        }

        tracing::warn!(
            "User {} is near expense limit: {}/{}",
            auth.user_uid,
            usage_payload.total_expenses,
            limits.max_expenses_per_month
        );
    }

    tx.commit().await.map_err(|e| {
        AppError::from_sqlx_error(e, "committing transaction for creating expense entry")
    })?;
    Ok(Json(response_data))
}

#[utoipa::path(get, path = "/expense-entries/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = ExpenseEntry)), tag = "Expense Entries", operation_id = "getExpenseEntry", security(("bearerAuth" = [])))]
pub async fn get_expense_entry(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
) -> Result<Json<ExpenseEntry>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::from_sqlx_error(e, "beginning transaction for getting expense entry")
    })?;
    let rec = ExpenseEntryRepo::get(&mut tx, uid).await?;
    group_guard(&auth, rec.group_uid, &state.db_pool).await?;
    tx.commit().await.map_err(|e| {
        AppError::from_sqlx_error(e, "committing transaction for getting expense entry")
    })?;
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
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateExpenseEntryPayload>,
) -> Result<Json<ExpenseEntry>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::from_sqlx_error(e, "beginning transaction for updating expense entry")
    })?;
    let prev_rec = ExpenseEntryRepo::get(&mut tx, uid).await?;
    group_guard(&auth, prev_rec.group_uid, &state.db_pool).await?;
    let updated = ExpenseEntryRepo::update(
        &mut tx,
        uid,
        UpdateExpenseEntryDbPayload {
            price: payload.price,
            product: payload.product,
            category_uid: payload.category_uid,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| {
        AppError::from_sqlx_error(e, "committing transaction for updating expense entry")
    })?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/expense-entries/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Expense Entries", operation_id = "deleteExpenseEntry", security(("bearerAuth" = [])))]
pub async fn delete_expense_entry(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(uid): Path<Uuid>,
) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::from_sqlx_error(e, "beginning transaction for deleting expense entry")
    })?;
    let prev_rec = ExpenseEntryRepo::get(&mut tx, uid).await?;
    group_guard(&auth, prev_rec.group_uid, &state.db_pool).await?;
    ExpenseEntryRepo::delete(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| {
        AppError::from_sqlx_error(e, "committing transaction for deleting expense entry")
    })?;
    Ok(())
}

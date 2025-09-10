
use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::info;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{error::app::AppError, repos::expense_entry::ExpenseEntry, types::AppState};



pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list_expense_entries).post(create_expense_entry))
        .route("/{uid}", axum::routing::get(get_expense_entry).put(update_expense_entry).delete(delete_expense_entry))
}

async fn list_expense_entries(
    State(state): State<AppState>
) -> Result<Json<Vec<ExpenseEntry>>, AppError> {
    let db_pool = &state.db_pool;
    let rows = sqlx::query_as(
        r#"
        SELECT uid, price, product, group_uid, category_uid, created_at, updated_at
        FROM expense_entries
        "#
    )    .fetch_all(db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct CreateExpenseEntryPayload {
    price: f64,
    product: String,
    group_uid: Uuid,
    category_uid: Uuid,
}

async fn create_expense_entry(State(state): State<AppState>, Json(payload): Json<CreateExpenseEntryPayload>) -> Result<Json<ExpenseEntry>, AppError> {
    let db_pool = &state.db_pool;
    let entry = ExpenseEntry {
        uid: Uuid::now_v7(),
        price: payload.price,
        product: payload.product,
        group_uid: payload.group_uid,
        category_uid: payload.category_uid,
        created_by: "system".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    sqlx::query(
        r#"
        INSERT INTO expense_entries (uid, price, product, group_uid, category_uid, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&entry.uid)
    .bind(&entry.price)
    .bind(&entry.product)
    .bind(&entry.group_uid)
    .bind(&entry.category_uid)
    .bind(&entry.created_at)
    .bind(&entry.updated_at)
    .execute(db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;
    Ok(Json(entry))
}

async fn get_expense_entry(Path(uid): Path<Uuid>) -> Result<Json<ExpenseEntry>, AppError> {
    info!("Fetching expense entry with uid: {}", uid);
    Ok(Json(ExpenseEntry::new()))
}

async fn update_expense_entry(Path(uid): Path<Uuid>) -> Result<Json<ExpenseEntry>, AppError> {
    info!("Updating expense entry with uid: {}", uid);
    Ok(Json(ExpenseEntry::new()))
}

async fn delete_expense_entry(Path(uid): Path<Uuid>) -> Result<(), AppError> {
    info!("Deleting expense entry with uid: {}", uid);
    Ok(())
}








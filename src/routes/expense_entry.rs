
use axum::{extract::Path, Json};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{error::AppError, types::AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExpenseEntry {
    pub uid: Uuid,
    pub price: f64,
    pub product: String,

    pub group_uid: Uuid,
    pub category_uid: u64,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExpenseEntry {
    fn new() -> Self {
        let time = Utc::now();
        ExpenseEntry {
            uid: Uuid::now_v7(),
            price: 0.0,
            product: String::new(),
            group_uid: Uuid::now_v7(),
            category_uid: 0,
            created_at: time,
            updated_at: time,
        }
    }
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list_expense_entries).post(create_expense_entry))
        .route("/{uid}", axum::routing::get(get_expense_entry).put(update_expense_entry).delete(delete_expense_entry))
}

async fn list_expense_entries() -> Result<Json<Vec<ExpenseEntry>>, AppError> {
    Ok(Json(vec![ExpenseEntry::new()]))
}

async fn create_expense_entry() -> Result<Json<ExpenseEntry>, AppError> {
    Ok(Json(ExpenseEntry::new()))
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








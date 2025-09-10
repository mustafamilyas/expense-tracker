use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::error::db::DatabaseError;

pub struct ExpenseEntryRepo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExpenseEntry {
    pub uid: Uuid,
    pub price: f64,
    pub product: String,
    pub created_by: String,

    pub group_uid: Uuid,
    pub category_uid: Uuid,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExpenseEntry {
    pub fn new() -> Self {
        let time = Utc::now();
        ExpenseEntry {
            uid: Uuid::now_v7(),
            price: 0.0,
            product: String::new(),
            created_by: "system".to_string(),

            group_uid: Uuid::now_v7(),
            category_uid: Uuid::now_v7(),
            created_at: time,
            updated_at: time,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateExpenseEntryPayload {
    pub price: f64,
    pub product: String,
    pub group_uid: Uuid,
    pub category_uid: Uuid,
}

// impl ExpenseEntryRepo {
//     pub async fn create_expense_entry(
//         db_pool: &sqlx::PgPool,
//         payload: CreateExpenseEntryPayload,
//     ) -> Result<ExpenseEntry, DatabaseError> {
//         let uid = uuid::Uuid::new_v4();
//         let rec = sqlx::query_as!(
//             ExpenseEntry,
//             r#"
//             INSERT INTO expense_entries (uid, price, product, group_uid, category_uid, created_by)
//             VALUES ($1, $2, $3, $4, $5, $6)
//             RETURNING uid, price, product, group_uid, category_uid, created_by, created_at, updated_at
//             "#,
//             uid,
//             payload.price,
//             payload.product,
//             payload.group_uid,
//             payload.category_uid,
//             "system" // Placeholder for created_by
//         )
//         .fetch_one(db_pool)
//         .await?;
//         Ok(rec)
//     }
// }
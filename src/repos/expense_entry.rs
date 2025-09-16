use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

pub struct ExpenseEntryRepo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
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
pub struct CreateExpenseEntryDbPayload {
    pub price: f64,
    pub product: String,
    pub group_uid: Uuid,
    pub category_uid: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExpenseEntryDbPayload {
    pub price: Option<f64>,
    pub product: Option<String>,
    pub category_uid: Option<Uuid>,
}

impl ExpenseEntryRepo {
    pub async fn create_expense_entry(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateExpenseEntryDbPayload,
    ) -> Result<ExpenseEntry, DatabaseError> {
        let uid = uuid::Uuid::new_v4();
        let rec = sqlx::query_as::<_, ExpenseEntry>(
            r#"
            INSERT INTO expense_entries (uid, price, product, group_uid, category_uid, created_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING uid, price, product, created_by, group_uid, category_uid, created_at, updated_at
            "#
        )
        .bind(uid)
        .bind(payload.price)
        .bind(payload.product)
        .bind(payload.group_uid)
        .bind(payload.category_uid)
        .bind("system")
        .fetch_one(tx.as_mut())
        .await?;
        Ok(rec)
    }

    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<ExpenseEntry>, DatabaseError> {
        let recs = sqlx::query_as::<_, ExpenseEntry>(
            r#"SELECT uid, price, product, created_by, group_uid, category_uid, created_at, updated_at
               FROM expense_entries ORDER BY created_at DESC"#
        )
        .fetch_all(tx.as_mut())
        .await?;
        Ok(recs)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<ExpenseEntry, DatabaseError> {
        let rec = sqlx::query_as::<_, ExpenseEntry>(
            r#"SELECT uid, price, product, created_by, group_uid, category_uid, created_at, updated_at
               FROM expense_entries WHERE uid = $1"#
        )
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(rec)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
        payload: UpdateExpenseEntryDbPayload,
    ) -> Result<ExpenseEntry, DatabaseError> {
        let current = Self::get(tx, uid).await?;
        let price = payload.price.unwrap_or(current.price);
        let product = payload.product.unwrap_or(current.product);
        let category_uid = payload.category_uid.unwrap_or(current.category_uid);
        let rec = sqlx::query_as::<_, ExpenseEntry>(
            r#"UPDATE expense_entries
               SET price = $1, product = $2, category_uid = $3, updated_at = now()
               WHERE uid = $4
               RETURNING uid, price, product, created_by, group_uid, category_uid, created_at, updated_at"#
        )
        .bind(price)
        .bind(product)
        .bind(category_uid)
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(rec)
    }

    pub async fn delete(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM expense_entries WHERE uid = $1")
            .bind(uid)
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }
}

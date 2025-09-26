use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::DatabaseError, repos::base::BaseRepo};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ExpenseGroup {
    pub uid: Uuid,
    pub name: String,
    pub owner: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateExpenseGroupDbPayload {
    pub name: String,
    pub owner: Uuid,
}

#[derive(Debug, Deserialize, serde::Serialize, ToSchema)]
pub struct UpdateExpenseGroupDbPayload {
    pub name: Option<String>,
}

pub struct ExpenseGroupRepo;

impl BaseRepo for ExpenseGroupRepo {
    fn get_table_name() -> &'static str {
        "expense_groups"
    }
}

impl ExpenseGroupRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<ExpenseGroup>, DatabaseError> {
        let query = format!(
            "SELECT uid, name, owner, created_at FROM {} ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, ExpenseGroup>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing expense groups"))?;
        Ok(rows)
    }

    pub async fn get_all_by_owner(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: Uuid,
    ) -> Result<Vec<ExpenseGroup>, DatabaseError> {
        let query = format!(
            "SELECT uid, name, owner, created_at FROM {} WHERE owner = $1 ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, ExpenseGroup>(&query)
            .bind(owner)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting expense groups by owner"))?;
        Ok(rows)
    }

    pub async fn count_by_owner(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: Uuid,
    ) -> Result<i64, DatabaseError> {
        let query = format!("SELECT COUNT(*) FROM {} WHERE owner = $1", Self::get_table_name());
        let count = sqlx::query_scalar::<_, i64>(&query)
            .bind(owner)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "counting expense groups by owner"))?;
        Ok(count)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<ExpenseGroup, DatabaseError> {
        let query = format!(
            "SELECT uid, name, owner, created_at FROM {} WHERE uid = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, ExpenseGroup>(&query)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting expense group"))?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateExpenseGroupDbPayload,
    ) -> Result<ExpenseGroup, DatabaseError> {
        let uid = Uuid::new_v4();
        let query = format!(
            "INSERT INTO {} (uid, name, owner) VALUES ($1, $2, $3) RETURNING uid, name, owner, created_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, ExpenseGroup>(&query)
            .bind(uid)
            .bind(payload.name)
            .bind(payload.owner)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating expense group"))?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
        payload: UpdateExpenseGroupDbPayload,
    ) -> Result<ExpenseGroup, DatabaseError> {
        let current = Self::get(tx, uid).await?;
        let name = payload.name.unwrap_or(current.name);
        let query = format!(
            "UPDATE {} SET name = $1 WHERE uid = $2 RETURNING uid, name, owner, created_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, ExpenseGroup>(&query)
            .bind(name)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating expense group"))?;
        Ok(row)
    }

    pub async fn delete(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<(), DatabaseError> {
        let query = format!("DELETE FROM {} WHERE uid = $1", Self::get_table_name());
        sqlx::query(&query)
            .bind(uid)
            .execute(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "deleting expense group"))?;
        Ok(())
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ExpenseGroup {
    pub uid: Uuid,
    pub name: String,
    pub owner: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateExpenseGroupPayload {
    pub name: String,
    pub owner: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateExpenseGroupPayload {
    pub name: Option<String>,
}

pub struct ExpenseGroupRepo;

impl ExpenseGroupRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<ExpenseGroup>, DatabaseError> {
        let rows = sqlx::query_as::<_, ExpenseGroup>(
            r#"SELECT uid, name, owner, created_at FROM expense_groups ORDER BY created_at DESC"#,
        )
        .fetch_all(tx.as_mut())
        .await?;
        Ok(rows)
    }

    pub async fn get_all_by_owner(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        owner: Uuid,
    ) -> Result<Vec<ExpenseGroup>, DatabaseError> {
        let rows = sqlx::query_as::<_, ExpenseGroup>(
            r#"SELECT uid, name, owner, created_at FROM expense_groups WHERE owner = $1 ORDER BY created_at DESC"#,
        )
        .bind(owner)
        .fetch_all(tx.as_mut())
        .await?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<ExpenseGroup, DatabaseError> {
        let row = sqlx::query_as::<_, ExpenseGroup>(
            r#"SELECT uid, name, owner, created_at FROM expense_groups WHERE uid = $1"#,
        )
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateExpenseGroupPayload,
    ) -> Result<ExpenseGroup, DatabaseError> {
        let uid = Uuid::new_v4();
        let row = sqlx::query_as::<_, ExpenseGroup>(
            r#"INSERT INTO expense_groups (uid, name, owner) VALUES ($1, $2, $3)
               RETURNING uid, name, owner, created_at"#,
        )
        .bind(uid)
        .bind(payload.name)
        .bind(payload.owner)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
        payload: UpdateExpenseGroupPayload,
    ) -> Result<ExpenseGroup, DatabaseError> {
        let current = Self::get(tx, uid).await?;
        let name = payload.name.unwrap_or(current.name);
        let row = sqlx::query_as::<_, ExpenseGroup>(
            r#"UPDATE expense_groups SET name = $1 WHERE uid = $2
               RETURNING uid, name, owner, created_at"#,
        )
        .bind(name)
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn delete(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM expense_groups WHERE uid = $1")
            .bind(uid)
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }
}

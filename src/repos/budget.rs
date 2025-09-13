use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::error::db::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Budget {
    pub uid: Uuid,
    pub group_uid: Uuid,
    pub category_uid: Uuid,
    pub amount: f64,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBudgetPayload {
    pub group_uid: Uuid,
    pub category_uid: Uuid,
    pub amount: f64,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBudgetPayload {
    pub amount: Option<f64>,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

pub struct BudgetRepo;

impl BudgetRepo {
    pub async fn list(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<Vec<Budget>, DatabaseError> {
        let rows = sqlx::query_as::<_, Budget>(
            r#"SELECT uid, group_uid, category_uid, amount, period_year, period_month FROM budgets
               ORDER BY group_uid, category_uid"#
        )
        .fetch_all(&mut *tx)
        .await?;
        Ok(rows)
    }

    pub async fn get(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, uid: Uuid) -> Result<Budget, DatabaseError> {
        let row = sqlx::query_as::<_, Budget>(
            r#"SELECT uid, group_uid, category_uid, amount, period_year, period_month FROM budgets WHERE uid = $1"#
        )
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;
        Ok(row)
    }

    pub async fn create(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, payload: CreateBudgetPayload) -> Result<Budget, DatabaseError> {
        let uid = Uuid::new_v4();
        let row = sqlx::query_as::<_, Budget>(
            r#"INSERT INTO budgets (uid, group_uid, category_uid, amount, period_year, period_month)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING uid, group_uid, category_uid, amount, period_year, period_month"#
        )
        .bind(uid)
        .bind(payload.group_uid)
        .bind(payload.category_uid)
        .bind(payload.amount)
        .bind(payload.period_year)
        .bind(payload.period_month)
        .fetch_one(&mut *tx)
        .await?;
        Ok(row)
    }

    pub async fn update(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, uid: Uuid, payload: UpdateBudgetPayload) -> Result<Budget, DatabaseError> {
        let current = Self::get(tx, uid).await?;
        let amount = payload.amount.unwrap_or(current.amount);
        let period_year = payload.period_year.or(current.period_year);
        let period_month = payload.period_month.or(current.period_month);
        let row = sqlx::query_as::<_, Budget>(
            r#"UPDATE budgets SET amount = $1, period_year = $2, period_month = $3 WHERE uid = $4
               RETURNING uid, group_uid, category_uid, amount, period_year, period_month"#
        )
        .bind(amount)
        .bind(period_year)
        .bind(period_month)
        .bind(uid)
        .fetch_one(&mut *tx)
        .await?;
        Ok(row)
    }

    pub async fn delete(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, uid: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM budgets WHERE uid = $1").bind(uid)
            .execute(&mut *tx)
            .await?;
        Ok(())
    }
}

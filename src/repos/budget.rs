use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::repos::base::BaseRepo;

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
pub struct CreateBudgetDbPayload {
    pub group_uid: Uuid,
    pub category_uid: Uuid,
    pub amount: f64,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBudgetDbPayload {
    pub amount: Option<f64>,
    pub period_year: Option<i32>,
    pub period_month: Option<i32>,
}

pub struct BudgetRepo;

impl BaseRepo for BudgetRepo {
    fn get_table_name() -> &'static str {
        "budgets"
    }
}

impl BudgetRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<Budget>, DatabaseError> {
        let query = format!(
            "SELECT uid, group_uid, category_uid, amount, period_year, period_month FROM {} ORDER BY group_uid, category_uid",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, Budget>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing budgets"))?;
        Ok(rows)
    }

    pub async fn list_by_group(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        group_uid: Uuid,
    ) -> Result<Vec<Budget>, DatabaseError> {
        let query = format!(
            "SELECT uid, group_uid, category_uid, amount, period_year, period_month FROM {} WHERE group_uid = $1 ORDER BY uid",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, Budget>(&query)
            .bind(group_uid)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing budgets"))?;
        Ok(rows)
    }

    pub async fn get_by_group_and_category(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        group_uid: Uuid,
        category_uid: Uuid,
    ) -> Result<Option<Budget>, DatabaseError> {
        let query = format!(
            "SELECT uid, group_uid, category_uid, amount, period_year, period_month FROM {} WHERE group_uid = $1 AND category_uid = $2",
            Self::get_table_name()
        );
        let budget = sqlx::query_as::<_, Budget>(&query)
            .bind(group_uid)
            .bind(category_uid)
            .fetch_optional(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting budget by group and category"))?;
        Ok(budget)
    }

    pub async fn count_by_group(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        group_uid: Uuid,
    ) -> Result<i64, DatabaseError> {
        let query = format!(
            "SELECT COUNT(*) FROM {} WHERE group_uid = $1",
            Self::get_table_name()
        );
        let count = sqlx::query_scalar::<_, i64>(&query)
            .bind(group_uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "counting budgets"))?;
        Ok(count)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<Budget, DatabaseError> {
        let query = format!(
            "SELECT uid, group_uid, category_uid, amount, period_year, period_month FROM {} WHERE uid = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Budget>(&query)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting budget"))?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateBudgetDbPayload,
    ) -> Result<Budget, DatabaseError> {
        let uid = Uuid::new_v4();
        let query = format!(
            "INSERT INTO {} (uid, group_uid, category_uid, amount, period_year, period_month) VALUES ($1, $2, $3, $4, $5, $6) RETURNING uid, group_uid, category_uid, amount, period_year, period_month",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Budget>(&query)
            .bind(uid)
            .bind(payload.group_uid)
            .bind(payload.category_uid)
            .bind(payload.amount)
            .bind(payload.period_year)
            .bind(payload.period_month)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating budget"))?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
        payload: UpdateBudgetDbPayload,
    ) -> Result<Budget, DatabaseError> {
        let current = Self::get(tx, uid).await?;
        let amount = payload.amount.unwrap_or(current.amount);
        let period_year = payload.period_year.or(current.period_year);
        let period_month = payload.period_month.or(current.period_month);
        let query = format!(
            "UPDATE {} SET amount = $1, period_year = $2, period_month = $3 WHERE uid = $4 RETURNING uid, group_uid, category_uid, amount, period_year, period_month",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Budget>(&query)
            .bind(amount)
            .bind(period_year)
            .bind(period_month)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating budget"))?;
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
            .map_err(|e| DatabaseError::from_sqlx_error(e, "deleting budget"))?;
        Ok(())
    }
}

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::repos::base::BaseRepo;
use crate::types::SubscriptionTier;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Subscription {
    pub id: Uuid,
    pub user_uid: Uuid,
    pub tier: SubscriptionTier,
    pub status: String,

    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Subscription {
    pub fn get_tier(&self) -> SubscriptionTier {
        self.tier.clone()
    }

    pub fn set_tier(&mut self, tier: SubscriptionTier) {
        self.tier = tier;
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSubscriptionDbPayload {
    pub user_uid: Uuid,
    pub tier: SubscriptionTier,
    pub status: Option<String>,

    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSubscriptionDbPayload {
    pub tier: Option<SubscriptionTier>,
    pub status: Option<String>,
    pub current_period_start: Option<Option<DateTime<Utc>>>,
    pub current_period_end: Option<Option<DateTime<Utc>>>,
    pub cancel_at_period_end: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserUsage {
    pub id: Uuid,
    pub user_uid: Uuid,
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub groups_count: i32,
    pub total_expenses: i32,
    pub total_members: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserUsageDbPayload {
    pub user_uid: Uuid,
    pub period_start: chrono::NaiveDate,
    pub period_end: chrono::NaiveDate,
    pub groups_count: i32,
    pub total_expenses: i32,
    pub total_members: i32,
}

pub struct SubscriptionRepo;

impl BaseRepo for SubscriptionRepo {
    fn get_table_name() -> &'static str {
        "subscriptions"
    }
}

impl SubscriptionRepo {
    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateSubscriptionDbPayload,
    ) -> Result<Subscription, DatabaseError> {
        let id = Uuid::new_v4();
        let status = payload.status.unwrap_or_else(|| "active".to_string());

        let query = format!(
            "INSERT INTO {} (id, user_uid, tier, status, current_period_start, current_period_end) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, user_uid, tier, status, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Subscription>(&query)
            .bind(id)
            .bind(payload.user_uid)
            .bind(payload.tier)
            .bind(status)
            .bind(payload.current_period_start)
            .bind(payload.current_period_end)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating subscription"))?;

        Ok(row)
    }

    pub async fn get_by_user(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_uid: Uuid,
    ) -> Result<Subscription, DatabaseError> {
        let query = format!(
            "SELECT id, user_uid, tier, status, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at FROM {} WHERE user_uid = $1 AND status = 'active' LIMIT 1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Subscription>(&query)
            .bind(user_uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting subscription by user"))?;

        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        payload: UpdateSubscriptionDbPayload,
    ) -> Result<Subscription, DatabaseError> {
        let current = Self::get(tx, id).await?;
        let tier = payload.tier.unwrap_or(current.get_tier());
        let status = payload.status.unwrap_or(current.status);
        let current_period_start = payload
            .current_period_start
            .unwrap_or(current.current_period_start);
        let current_period_end = payload
            .current_period_end
            .unwrap_or(current.current_period_end);
        let cancel_at_period_end = payload
            .cancel_at_period_end
            .unwrap_or(current.cancel_at_period_end);

        let query = format!(
            "UPDATE {} SET tier = $1, status = $2, current_period_start = $3, current_period_end = $4, cancel_at_period_end = $5 WHERE id = $6 RETURNING id, user_uid, tier, status, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Subscription>(&query)
            .bind(tier)
            .bind(status)
            .bind(current_period_start)
            .bind(current_period_end)
            .bind(cancel_at_period_end)
            .bind(id)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating subscription"))?;

        Ok(row)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> Result<Subscription, DatabaseError> {
        let query = format!(
            "SELECT id, user_uid, tier, status, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at FROM {} WHERE id = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Subscription>(&query)
            .bind(id)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting subscription"))?;

        Ok(row)
    }

    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<Subscription>, DatabaseError> {
        let query = format!(
            "SELECT id, user_uid, tier, status, current_period_start, current_period_end, cancel_at_period_end, created_at, updated_at FROM {} ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, Subscription>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing subscriptions"))?;

        Ok(rows)
    }
}

pub struct UserUsageRepo;

impl BaseRepo for UserUsageRepo {
    fn get_table_name() -> &'static str {
        "user_usage"
    }
}

impl UserUsageRepo {
    pub async fn create_or_update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateUserUsageDbPayload,
    ) -> Result<UserUsage, DatabaseError> {
        let query = format!(
            "INSERT INTO {} (user_uid, period_start, period_end, groups_count, total_expenses, total_members) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (user_uid, period_start, period_end) DO UPDATE SET groups_count = EXCLUDED.groups_count, total_expenses = EXCLUDED.total_expenses, total_members = EXCLUDED.total_members, updated_at = NOW() RETURNING id, user_uid, period_start, period_end, groups_count, total_expenses, total_members, created_at, updated_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, UserUsage>(&query)
            .bind(payload.user_uid)
            .bind(payload.period_start)
            .bind(payload.period_end)
            .bind(payload.groups_count)
            .bind(payload.total_expenses)
            .bind(payload.total_members)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating or updating user usage"))?;

        Ok(row)
    }

    pub async fn get_current_usage(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_uid: Uuid,
    ) -> Result<UserUsage, DatabaseError> {
        let now = Utc::now().date_naive();
        let period_start = now.with_day(1).unwrap(); // First day of current month
        let period_end = if now.month() == 12 {
            chrono::NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap()
        } else {
            chrono::NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap()
        };

        let query = format!(
            "SELECT id, user_uid, period_start, period_end, groups_count, total_expenses, total_members, created_at, updated_at FROM {} WHERE user_uid = $1 AND period_start = $2 AND period_end = $3",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, UserUsage>(&query)
            .bind(user_uid)
            .bind(period_start)
            .bind(period_end)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting current user usage"))?;

        Ok(row)
    }

    pub async fn calculate_current_usage(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_uid: Uuid,
    ) -> Result<CreateUserUsageDbPayload, DatabaseError> {
        let now = Utc::now().date_naive();
        let period_start = now.with_day(1).unwrap();
        let period_end = if now.month() == 12 {
            chrono::NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap()
        } else {
            chrono::NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap()
        };

        // Count groups
        let groups_count = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(DISTINCT gm.group_uid)
               FROM group_members gm
               WHERE gm.user_uid = $1"#,
        )
        .bind(user_uid)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|e| DatabaseError::from_sqlx_error(e, "counting groups for user"))?;

        // Count expenses this month
        let total_expenses = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(*)
               FROM expense_entries e
               JOIN group_members gm ON e.group_uid = gm.group_uid
               WHERE gm.user_uid = $1 AND e.created_at >= $2 AND e.created_at < $3"#,
        )
        .bind(user_uid)
        .bind(period_start.and_hms_opt(0, 0, 0).unwrap().and_utc())
        .bind(period_end.and_hms_opt(0, 0, 0).unwrap().and_utc())
        .fetch_one(tx.as_mut())
        .await
        .map_err(|e| DatabaseError::from_sqlx_error(e, "counting expenses for user"))?;

        // Count total members across all groups
        let total_members = sqlx::query_scalar::<_, i64>(
            r#"SELECT COUNT(DISTINCT gm2.user_uid)
               FROM group_members gm1
               JOIN group_members gm2 ON gm1.group_uid = gm2.group_uid
               WHERE gm1.user_uid = $1"#,
        )
        .bind(user_uid)
        .fetch_one(tx.as_mut())
        .await
        .map_err(|e| DatabaseError::from_sqlx_error(e, "counting members for user"))?;

        Ok(CreateUserUsageDbPayload {
            user_uid,
            period_start,
            period_end,
            groups_count: groups_count as i32,
            total_expenses: total_expenses as i32,
            total_members: total_members as i32,
        })
    }
}

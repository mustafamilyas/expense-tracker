use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::repos::base::BaseRepo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ChatBinding {
    pub id: Uuid,
    pub group_uid: Uuid,
    pub platform: String, // from enum via ::text
    pub p_uid: String,
    pub status: String, // from enum via ::text
    pub bound_by: Uuid,
    pub bound_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatBindingDbPayload {
    pub group_uid: Uuid,
    pub platform: String,
    pub p_uid: String,
    pub status: Option<String>,
    pub bound_by: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChatBindingDbPayload {
    pub status: Option<String>,
    pub revoked_at: Option<Option<DateTime<Utc>>>,
}

pub struct ChatBindingRepo;

impl BaseRepo for ChatBindingRepo {
    fn get_table_name() -> &'static str {
        "chat_bindings"
    }
}

impl ChatBindingRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<ChatBinding>, DatabaseError> {
        let query = format!(
            "SELECT id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at FROM {} ORDER BY bound_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, ChatBinding>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing chat bindings"))?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> Result<ChatBinding, DatabaseError> {
        let query = format!(
            "SELECT id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at FROM {} WHERE id = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, ChatBinding>(&query)
            .bind(id)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting chat binding"))?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateChatBindingDbPayload,
    ) -> Result<ChatBinding, DatabaseError> {
        let id = Uuid::new_v4();
        let query = format!(
            "INSERT INTO {} (id, group_uid, platform, p_uid, status, bound_by) VALUES ($1, $2, CAST($3 AS chat_platform), $4, COALESCE(CAST($5 AS binding_status), 'active'::binding_status), $6) RETURNING id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, ChatBinding>(&query)
            .bind(id)
            .bind(payload.group_uid)
            .bind(payload.platform)
            .bind(payload.p_uid)
            .bind(payload.status)
            .bind(payload.bound_by)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating chat binding"))?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        payload: UpdateChatBindingDbPayload,
    ) -> Result<ChatBinding, DatabaseError> {
        let current = Self::get(tx, id).await?;
        let status = payload.status.unwrap_or(current.status);
        let revoked_at = match payload.revoked_at {
            Some(v) => v,
            None => current.revoked_at,
        };
        let query = format!(
            "UPDATE {} SET status = CAST($1 AS binding_status), revoked_at = $2 WHERE id = $3 RETURNING id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, ChatBinding>(&query)
            .bind(status)
            .bind(revoked_at)
            .bind(id)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating chat binding"))?;
        Ok(row)
    }

    pub async fn delete(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> Result<(), DatabaseError> {
        let query = format!("DELETE FROM {} WHERE id = $1", Self::get_table_name());
        sqlx::query(&query)
            .bind(id)
            .execute(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "deleting chat binding"))?;
        Ok(())
    }
}

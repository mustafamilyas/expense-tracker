use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::error::db::DatabaseError;

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
pub struct CreateChatBindingPayload {
    pub group_uid: Uuid,
    pub platform: String,
    pub p_uid: String,
    pub status: Option<String>,
    pub bound_by: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChatBindingPayload {
    pub status: Option<String>,
    pub revoked_at: Option<Option<DateTime<Utc>>>,
}

pub struct ChatBindingRepo;

impl ChatBindingRepo {
    pub async fn list(db: &sqlx::PgPool) -> Result<Vec<ChatBinding>, DatabaseError> {
        let rows = sqlx::query_as::<_, ChatBinding>(
            r#"SELECT id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at
               FROM chat_bindings ORDER BY bound_at DESC"#
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get(db: &sqlx::PgPool, id: Uuid) -> Result<ChatBinding, DatabaseError> {
        let row = sqlx::query_as::<_, ChatBinding>(
            r#"SELECT id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at
               FROM chat_bindings WHERE id = $1"#
        )
        .bind(id)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn create(db: &sqlx::PgPool, payload: CreateChatBindingPayload) -> Result<ChatBinding, DatabaseError> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, ChatBinding>(
            r#"INSERT INTO chat_bindings (id, group_uid, platform, p_uid, status, bound_by)
               VALUES ($1, $2, CAST($3 AS chat_platform), $4, COALESCE(CAST($5 AS binding_status), 'active'::binding_status), $6)
               RETURNING id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at"#
        )
        .bind(id)
        .bind(payload.group_uid)
        .bind(payload.platform)
        .bind(payload.p_uid)
        .bind(payload.status)
        .bind(payload.bound_by)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn update(db: &sqlx::PgPool, id: Uuid, payload: UpdateChatBindingPayload) -> Result<ChatBinding, DatabaseError> {
        let current = Self::get(db, id).await?;
        let status = payload.status.unwrap_or(current.status);
        let revoked_at = match payload.revoked_at { Some(v) => v, None => current.revoked_at };
        let row = sqlx::query_as::<_, ChatBinding>(
            r#"UPDATE chat_bindings SET status = CAST($1 AS binding_status), revoked_at = $2 WHERE id = $3
               RETURNING id, group_uid, platform::text as platform, p_uid, status::text as status, bound_by, bound_at, revoked_at"#
        )
        .bind(status)
        .bind(revoked_at)
        .bind(id)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(db: &sqlx::PgPool, id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM chat_bindings WHERE id = $1").bind(id)
            .execute(db)
            .await?;
        Ok(())
    }
}

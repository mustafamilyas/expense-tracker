use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ChatBindRequest {
    pub id: Uuid,
    pub platform: String, // from enum via ::text
    pub p_uid: String,
    pub nonce: String,
    pub user_uid: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateChatBindRequestDbPayload {
    pub platform: String,
    pub p_uid: String,
    pub nonce: String,
    pub user_uid: Option<Uuid>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChatBindRequestDbPayload {
    pub user_uid: Option<Option<Uuid>>, // Some(None) to clear, Some(Some(v)) to set
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct ChatBindRequestRepo;

impl ChatBindRequestRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<ChatBindRequest>, DatabaseError> {
        let rows = sqlx::query_as::<_, ChatBindRequest>(
            r#"SELECT id, platform::text as platform, p_uid, nonce, user_uid, expires_at, created_at
               FROM chat_bind_requests ORDER BY created_at DESC"#,
        )
        .fetch_all(tx.as_mut())
        .await?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> Result<ChatBindRequest, DatabaseError> {
        let row = sqlx::query_as::<_, ChatBindRequest>(
            r#"SELECT id, platform::text as platform, p_uid, nonce, user_uid, expires_at, created_at
               FROM chat_bind_requests WHERE id = $1"#,
        )
        .bind(id)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateChatBindRequestDbPayload,
    ) -> Result<ChatBindRequest, DatabaseError> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, ChatBindRequest>(
            r#"INSERT INTO chat_bind_requests (id, platform, p_uid, nonce, user_uid, expires_at)
               VALUES ($1, CAST($2 AS chat_platform), $3, $4, $5, $6)
               RETURNING id, platform::text as platform, p_uid, nonce, user_uid, expires_at, created_at"#
        )
        .bind(id)
        .bind(payload.platform)
        .bind(payload.p_uid)
        .bind(payload.nonce)
        .bind(payload.user_uid)
        .bind(payload.expires_at)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        payload: UpdateChatBindRequestDbPayload,
    ) -> Result<ChatBindRequest, DatabaseError> {
        let current = Self::get(tx, id).await?;
        let user_uid = match payload.user_uid {
            Some(u) => u,
            None => current.user_uid,
        };
        let expires_at = payload.expires_at.unwrap_or(current.expires_at);
        let row = sqlx::query_as::<_, ChatBindRequest>(
            r#"UPDATE chat_bind_requests SET user_uid = $1, expires_at = $2 WHERE id = $3
               RETURNING id, platform::text as platform, p_uid, nonce, user_uid, expires_at, created_at"#
        )
        .bind(user_uid)
        .bind(expires_at)
        .bind(id)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn delete(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM chat_bind_requests WHERE id = $1")
            .bind(id)
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }
}

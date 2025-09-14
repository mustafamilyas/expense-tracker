use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::error::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GroupMember {
    pub id: Uuid,
    pub group_uid: Uuid,
    pub user_uid: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupMemberPayload {
    pub group_uid: Uuid,
    pub user_uid: Uuid,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupMemberPayload {
    pub role: Option<String>,
}

pub struct GroupMemberRepo;

impl GroupMemberRepo {
    pub async fn list(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<Vec<GroupMember>, DatabaseError> {
        let rows = sqlx::query_as::<_, GroupMember>(
            r#"SELECT id, group_uid, user_uid, role, created_at FROM group_members ORDER BY created_at DESC"#
        )
        .fetch_all(tx.as_mut())
        .await?;
        Ok(rows)
    }

    pub async fn get(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, id: Uuid) -> Result<GroupMember, DatabaseError> {
        let row = sqlx::query_as::<_, GroupMember>(
            r#"SELECT id, group_uid, user_uid, role, created_at FROM group_members WHERE id = $1"#
        )
        .bind(id)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn create(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, payload: CreateGroupMemberPayload) -> Result<GroupMember, DatabaseError> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, GroupMember>(
            r#"INSERT INTO group_members (id, group_uid, user_uid, role)
               VALUES ($1, $2, $3, $4)
               RETURNING id, group_uid, user_uid, role, created_at"#
        )
        .bind(id)
        .bind(payload.group_uid)
        .bind(payload.user_uid)
        .bind(payload.role)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn update(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, id: Uuid, payload: UpdateGroupMemberPayload) -> Result<GroupMember, DatabaseError> {
        let current = Self::get(tx, id).await?;
        let role = payload.role.unwrap_or(current.role);
        let row = sqlx::query_as::<_, GroupMember>(
            r#"UPDATE group_members SET role = $1 WHERE id = $2
               RETURNING id, group_uid, user_uid, role, created_at"#
        )
        .bind(role)
        .bind(id)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn delete(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, id: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM group_members WHERE id = $1").bind(id)
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }
}

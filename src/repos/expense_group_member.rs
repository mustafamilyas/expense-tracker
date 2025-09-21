use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::repos::base::BaseRepo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GroupMember {
    pub id: Uuid,
    pub group_uid: Uuid,
    pub user_uid: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupMemberDbPayload {
    pub group_uid: Uuid,
    pub user_uid: Uuid,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGroupMemberDbPayload {
    pub role: Option<String>,
}

pub struct GroupMemberRepo;

impl BaseRepo for GroupMemberRepo {
    fn get_table_name() -> &'static str {
        "group_members"
    }
}

impl GroupMemberRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<GroupMember>, DatabaseError> {
        let query = format!(
            "SELECT id, group_uid, user_uid, role, created_at FROM {} ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, GroupMember>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing group members"))?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
    ) -> Result<GroupMember, DatabaseError> {
        let query = format!(
            "SELECT id, group_uid, user_uid, role, created_at FROM {} WHERE id = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, GroupMember>(&query)
            .bind(id)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting group member"))?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateGroupMemberDbPayload,
    ) -> Result<GroupMember, DatabaseError> {
        let id = Uuid::new_v4();
        let query = format!(
            "INSERT INTO {} (id, group_uid, user_uid, role) VALUES ($1, $2, $3, $4) RETURNING id, group_uid, user_uid, role, created_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, GroupMember>(&query)
            .bind(id)
            .bind(payload.group_uid)
            .bind(payload.user_uid)
            .bind(payload.role)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating group member"))?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        id: Uuid,
        payload: UpdateGroupMemberDbPayload,
    ) -> Result<GroupMember, DatabaseError> {
        let current = Self::get(tx, id).await?;
        let role = payload.role.unwrap_or(current.role);
        let query = format!(
            "UPDATE {} SET role = $1 WHERE id = $2 RETURNING id, group_uid, user_uid, role, created_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, GroupMember>(&query)
            .bind(role)
            .bind(id)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating group member"))?;
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
            .map_err(|e| DatabaseError::from_sqlx_error(e, "deleting group member"))?;
        Ok(())
    }
}

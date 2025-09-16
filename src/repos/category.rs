use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Category {
    pub uid: Uuid,
    pub group_uid: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryDbPayload {
    pub group_uid: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryDbPayload {
    pub name: Option<String>,
    pub description: Option<String>,
}

pub struct CategoryRepo;

impl CategoryRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<Category>, DatabaseError> {
        let rows = sqlx::query_as::<_, Category>(
            r#"SELECT uid, group_uid, name, description, created_at, updated_at FROM categories ORDER BY created_at DESC"#
        )
        .fetch_all(tx.as_mut())
        .await?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<Category, DatabaseError> {
        let row = sqlx::query_as::<_, Category>(
            r#"SELECT uid, group_uid, name, description, created_at, updated_at FROM categories WHERE uid = $1"#
        )
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateCategoryDbPayload,
    ) -> Result<Category, DatabaseError> {
        let uid = Uuid::new_v4();
        let row = sqlx::query_as::<_, Category>(
            r#"INSERT INTO categories (uid, group_uid, name, description)
               VALUES ($1, $2, $3, $4)
               RETURNING uid, group_uid, name, description, created_at, updated_at"#,
        )
        .bind(uid)
        .bind(payload.group_uid)
        .bind(payload.name)
        .bind(payload.description)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
        payload: UpdateCategoryDbPayload,
    ) -> Result<Category, DatabaseError> {
        let current = Self::get(tx, uid).await?;
        let name = payload.name.unwrap_or(current.name);
        let description = payload.description.or(current.description);
        let row = sqlx::query_as::<_, Category>(
            r#"UPDATE categories SET name = $1, description = $2 WHERE uid = $3
               RETURNING uid, group_uid, name, description, created_at, updated_at"#,
        )
        .bind(name)
        .bind(description)
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn delete(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM categories WHERE uid = $1")
            .bind(uid)
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }
}

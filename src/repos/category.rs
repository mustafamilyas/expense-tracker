use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::error::db::DatabaseError;

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
pub struct CreateCategoryPayload {
    pub group_uid: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryPayload {
    pub name: Option<String>,
    pub description: Option<String>,
}

pub struct CategoryRepo;

impl CategoryRepo {
    pub async fn list(db: &sqlx::PgPool) -> Result<Vec<Category>, DatabaseError> {
        let rows = sqlx::query_as::<_, Category>(
            r#"SELECT uid, group_uid, name, description, created_at, updated_at FROM categories ORDER BY created_at DESC"#
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get(db: &sqlx::PgPool, uid: Uuid) -> Result<Category, DatabaseError> {
        let row = sqlx::query_as::<_, Category>(
            r#"SELECT uid, group_uid, name, description, created_at, updated_at FROM categories WHERE uid = $1"#
        )
        .bind(uid)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn create(db: &sqlx::PgPool, payload: CreateCategoryPayload) -> Result<Category, DatabaseError> {
        let uid = Uuid::new_v4();
        let row = sqlx::query_as::<_, Category>(
            r#"INSERT INTO categories (uid, group_uid, name, description)
               VALUES ($1, $2, $3, $4)
               RETURNING uid, group_uid, name, description, created_at, updated_at"#
        )
        .bind(uid)
        .bind(payload.group_uid)
        .bind(payload.name)
        .bind(payload.description)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn update(db: &sqlx::PgPool, uid: Uuid, payload: UpdateCategoryPayload) -> Result<Category, DatabaseError> {
        let current = Self::get(db, uid).await?;
        let name = payload.name.unwrap_or(current.name);
        let description = payload.description.or(current.description);
        let row = sqlx::query_as::<_, Category>(
            r#"UPDATE categories SET name = $1, description = $2 WHERE uid = $3
               RETURNING uid, group_uid, name, description, created_at, updated_at"#
        )
        .bind(name)
        .bind(description)
        .bind(uid)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(db: &sqlx::PgPool, uid: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM categories WHERE uid = $1").bind(uid)
            .execute(db)
            .await?;
        Ok(())
    }
}

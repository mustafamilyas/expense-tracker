use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::repos::base::BaseRepo;

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

impl BaseRepo for CategoryRepo {
    fn get_table_name() -> &'static str {
        "categories"
    }
}

impl CategoryRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<Category>, DatabaseError> {
        let query = format!(
            "SELECT uid, group_uid, name, description,  created_at, updated_at FROM {} ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, Category>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing categories"))?;
        Ok(rows)
    }

    pub async fn list_by_group(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        group_uid: Uuid,
    ) -> Result<Vec<Category>, DatabaseError> {
        let query = format!(
            "SELECT uid, group_uid, name, description,  created_at, updated_at FROM {} WHERE group_uid = $1 ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, Category>(&query)
            .bind(group_uid)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing categories by group"))?;
        Ok(rows)
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
            .map_err(|e| DatabaseError::from_sqlx_error(e, "counting categories by group"))?;
        Ok(count)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<Category, DatabaseError> {
        let query = format!(
            "SELECT uid, group_uid, name, description,  created_at, updated_at FROM {} WHERE uid = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Category>(&query)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting category"))?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateCategoryDbPayload,
    ) -> Result<Category, DatabaseError> {
        let uid = Uuid::new_v4();
        let query = format!(
            "INSERT INTO {} (uid, group_uid, name, description) VALUES ($1, $2, $3, $4) RETURNING uid, group_uid, name, description, created_at, updated_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Category>(&query)
            .bind(uid)
            .bind(payload.group_uid)
            .bind(payload.name)
            .bind(payload.description)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating category"))?;
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
        let query = format!(
            "UPDATE {} SET name = $1, description = $2 WHERE uid = $3 RETURNING uid, group_uid, name, description, created_at, updated_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, Category>(&query)
            .bind(name)
            .bind(description)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating category"))?;
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
            .map_err(|e| DatabaseError::from_sqlx_error(e, "deleting category"))?;
        Ok(())
    }

    pub async fn find_by_name_or_alias(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        group_uid: Uuid,
        name_or_alias: &str,
    ) -> Result<Option<Category>, DatabaseError> {
        // First check if it's a category name
        let query = format!(
            "SELECT uid, group_uid, name, description, created_at, updated_at FROM {} WHERE group_uid = $1 AND name = $2",
            Self::get_table_name()
        );
        if let Ok(category) = sqlx::query_as::<_, Category>(&query)
            .bind(group_uid)
            .bind(name_or_alias)
            .fetch_one(tx.as_mut())
            .await
        {
            return Ok(Some(category));
        }

        // Then check aliases
        let query = format!(
            "SELECT c.uid, c.group_uid, c.name, c.description, c.created_at, c.updated_at FROM {} c JOIN categories_aliases ca ON c.uid = ca.category_uid WHERE ca.group_uid = $1 AND ca.alias = $2",
            Self::get_table_name()
        );
        let category = sqlx::query_as::<_, Category>(&query)
            .bind(group_uid)
            .bind(name_or_alias)
            .fetch_optional(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "finding category by name or alias"))?;
        Ok(category)
    }
}

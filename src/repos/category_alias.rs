use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::error::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CategoryAlias {
    pub alias_uid: Uuid,
    pub group_uid: Uuid,
    pub alias: String,
    pub category_uid: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryAliasPayload {
    pub group_uid: Uuid,
    pub alias: String,
    pub category_uid: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryAliasPayload {
    pub alias: Option<String>,
    pub category_uid: Option<Uuid>,
}

pub struct CategoryAliasRepo;

impl CategoryAliasRepo {
    pub async fn list(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>) -> Result<Vec<CategoryAlias>, DatabaseError> {
        let rows = sqlx::query_as::<_, CategoryAlias>(
            r#"SELECT alias_uid, group_uid, alias, category_uid FROM categories_aliases ORDER BY alias"#
        )
        .fetch_all(tx.as_mut())
        .await?;
        Ok(rows)
    }

    pub async fn get(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, alias_uid: Uuid) -> Result<CategoryAlias, DatabaseError> {
        let row = sqlx::query_as::<_, CategoryAlias>(
            r#"SELECT alias_uid, group_uid, alias, category_uid FROM categories_aliases WHERE alias_uid = $1"#
        )
        .bind(alias_uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn create(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, payload: CreateCategoryAliasPayload) -> Result<CategoryAlias, DatabaseError> {
        let alias_uid = Uuid::new_v4();
        let row = sqlx::query_as::<_, CategoryAlias>(
            r#"INSERT INTO categories_aliases (alias_uid, group_uid, alias, category_uid)
               VALUES ($1, $2, $3, $4)
               RETURNING alias_uid, group_uid, alias, category_uid"#
        )
        .bind(alias_uid)
        .bind(payload.group_uid)
        .bind(payload.alias)
        .bind(payload.category_uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn update(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, alias_uid: Uuid, payload: UpdateCategoryAliasPayload) -> Result<CategoryAlias, DatabaseError> {
        let current = Self::get(tx, alias_uid).await?;
        let alias = payload.alias.unwrap_or(current.alias);
        let category_uid = payload.category_uid.unwrap_or(current.category_uid);
        let row = sqlx::query_as::<_, CategoryAlias>(
            r#"UPDATE categories_aliases SET alias = $1, category_uid = $2 WHERE alias_uid = $3
               RETURNING alias_uid, group_uid, alias, category_uid"#
        )
        .bind(alias)
        .bind(category_uid)
        .bind(alias_uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn delete(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, alias_uid: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM categories_aliases WHERE alias_uid = $1").bind(alias_uid)
            .execute(tx.as_mut())
            .await?;
        Ok(())
    }
}

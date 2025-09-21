use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::repos::base::BaseRepo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CategoryAlias {
    pub alias_uid: Uuid,
    pub group_uid: Uuid,
    pub alias: String,
    pub category_uid: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryAliasDbPayload {
    pub group_uid: Uuid,
    pub alias: String,
    pub category_uid: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoryAliasDbPayload {
    pub alias: Option<String>,
    pub category_uid: Option<Uuid>,
}

pub struct CategoryAliasRepo;

impl BaseRepo for CategoryAliasRepo {
    fn get_table_name() -> &'static str {
        "categories_aliases"
    }
}

impl CategoryAliasRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<CategoryAlias>, DatabaseError> {
        let query = format!(
            "SELECT alias_uid, group_uid, alias, category_uid FROM {} ORDER BY alias",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, CategoryAlias>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing category aliases"))?;
        Ok(rows)
    }

    pub async fn list_by_category(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        category_uid: Uuid,
    ) -> Result<Vec<CategoryAlias>, DatabaseError> {
        let query = format!(
            "SELECT alias_uid, group_uid, alias, category_uid FROM {} WHERE category_uid = $1 ORDER BY alias",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, CategoryAlias>(&query)
            .bind(category_uid)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| {
                DatabaseError::from_sqlx_error(e, "listing category aliases by category")
            })?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        alias_uid: Uuid,
    ) -> Result<CategoryAlias, DatabaseError> {
        let query = format!(
            "SELECT alias_uid, group_uid, alias, category_uid FROM {} WHERE alias_uid = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, CategoryAlias>(&query)
            .bind(alias_uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting category alias"))?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateCategoryAliasDbPayload,
    ) -> Result<CategoryAlias, DatabaseError> {
        let alias_uid = Uuid::new_v4();
        let query = format!(
            "INSERT INTO {} (alias_uid, group_uid, alias, category_uid) VALUES ($1, $2, $3, $4) RETURNING alias_uid, group_uid, alias, category_uid",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, CategoryAlias>(&query)
            .bind(alias_uid)
            .bind(payload.group_uid)
            .bind(payload.alias)
            .bind(payload.category_uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating category alias"))?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        alias_uid: Uuid,
        payload: UpdateCategoryAliasDbPayload,
    ) -> Result<CategoryAlias, DatabaseError> {
        let current = Self::get(tx, alias_uid).await?;
        let alias = payload.alias.unwrap_or(current.alias);
        let category_uid = payload.category_uid.unwrap_or(current.category_uid);
        let query = format!(
            "UPDATE {} SET alias = $1, category_uid = $2 WHERE alias_uid = $3 RETURNING alias_uid, group_uid, alias, category_uid",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, CategoryAlias>(&query)
            .bind(alias)
            .bind(category_uid)
            .bind(alias_uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating category alias"))?;
        Ok(row)
    }

    pub async fn delete(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        alias_uid: Uuid,
    ) -> Result<(), DatabaseError> {
        let query = format!(
            "DELETE FROM {} WHERE alias_uid = $1",
            Self::get_table_name()
        );
        sqlx::query(&query)
            .bind(alias_uid)
            .execute(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "deleting category alias"))?;
        Ok(())
    }
}

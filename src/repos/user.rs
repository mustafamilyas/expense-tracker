use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;
use crate::repos::base::BaseRepo;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub uid: Uuid,
    pub email: String,
    pub phash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserDbPayload {
    pub email: String,
    pub phash: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserDbPayload {
    pub email: Option<String>,
    pub phash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserRead {
    pub uid: Uuid,
    pub email: String,
}

pub struct UserRepo;

impl BaseRepo for UserRepo {
    fn get_table_name() -> &'static str {
        "users"
    }
}

impl UserRepo {
    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<UserRead>, DatabaseError> {
        let query = format!(
            "SELECT uid, email FROM {} ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, UserRead>(&query)
            .fetch_all(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "listing users"))?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<UserRead, DatabaseError> {
        let query = format!(
            "SELECT uid, email FROM {} WHERE uid = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, UserRead>(&query)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting user"))?;
        Ok(row)
    }

    pub async fn get_full(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<User, DatabaseError> {
        let query = format!(
            "SELECT uid, email, phash, created_at FROM {} WHERE uid = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, User>(&query)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting full user"))?;
        Ok(row)
    }

    pub async fn get_by_email(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        email: &str,
    ) -> Result<User, DatabaseError> {
        let query = format!(
            "SELECT uid, email, phash, created_at FROM {} WHERE email = $1",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, User>(&query)
            .bind(email)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "getting user by email"))?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateUserDbPayload,
    ) -> Result<User, DatabaseError> {
        let uid = Uuid::new_v4();
        let query = format!(
            "INSERT INTO {} (uid, email, phash) VALUES ($1, $2, $3) RETURNING uid, email, phash, created_at",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, User>(&query)
            .bind(uid)
            .bind(payload.email)
            .bind(payload.phash)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "creating user"))?;
        Ok(row)
    }

    pub async fn update(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
        payload: UpdateUserDbPayload,
    ) -> Result<UserRead, DatabaseError> {
        let current = Self::get_full(tx, uid).await?;
        let email = payload.email.unwrap_or(current.email);
        let phash = payload.phash.unwrap_or(current.phash);
        let query = format!(
            "UPDATE {} SET email = $1, phash = $2 WHERE uid = $3 RETURNING uid, email",
            Self::get_table_name()
        );
        let row = sqlx::query_as::<_, UserRead>(&query)
            .bind(email)
            .bind(phash)
            .bind(uid)
            .fetch_one(tx.as_mut())
            .await
            .map_err(|e| DatabaseError::from_sqlx_error(e, "updating user"))?;
        Ok(row)
    }
}

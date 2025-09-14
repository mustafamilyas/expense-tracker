use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub uid: Uuid,
    pub email: String,
    pub phash: String,
    pub created_at: DateTime<Utc>,
    pub start_over_date: i16,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserDbPayload {
    pub email: String,
    pub phash: String,
    pub start_over_date: i16,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserDbPayload {
    pub email: Option<String>,
    pub phash: Option<String>,
    pub start_over_date: Option<i16>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserRead {
    pub uid: Uuid,
    pub email: String,
    pub start_over_date: i16,
}

pub struct UserRepo;

impl UserRepo {
    pub fn get_table_name() -> &'static str {
        "users"
    }

    pub async fn list(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Result<Vec<UserRead>, DatabaseError> {
        let query = format!(
            "SELECT uid, email, start_over_date FROM {} ORDER BY created_at DESC",
            Self::get_table_name()
        );
        let rows = sqlx::query_as::<_, UserRead>(&query)
            .fetch_all(tx.as_mut())
            .await?;
        Ok(rows)
    }

    pub async fn get(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<UserRead, DatabaseError> {
        let row = sqlx::query_as::<_, UserRead>(
            r#"SELECT uid, email, start_over_date FROM users WHERE uid = $1"#,
        )
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn get_full(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        uid: Uuid,
    ) -> Result<User, DatabaseError> {
        let row = sqlx::query_as::<_, User>(
            r#"SELECT uid, email, phash, created_at, start_over_date FROM users WHERE uid = $1"#,
        )
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn get_by_email(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        email: &str,
    ) -> Result<User, DatabaseError> {
        let row = sqlx::query_as::<_, User>(
            r#"SELECT uid, email, phash, created_at, start_over_date FROM users WHERE email = $1"#,
        )
        .bind(email)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }

    pub async fn create(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        payload: CreateUserDbPayload,
    ) -> Result<User, DatabaseError> {
        let uid = Uuid::new_v4();
        let row = sqlx::query_as::<_, User>(
            r#"INSERT INTO users (uid, email, phash, start_over_date)
               VALUES ($1, $2, $3, $4)
               RETURNING uid, email, phash, created_at, start_over_date"#,
        )
        .bind(uid)
        .bind(payload.email)
        .bind(payload.phash)
        .bind(payload.start_over_date)
        .fetch_one(tx.as_mut())
        .await?;
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
        let start_over_date = payload.start_over_date.unwrap_or(current.start_over_date);
        let row = sqlx::query_as::<_, UserRead>(
            r#"UPDATE users SET email = $1, phash = $2, start_over_date = $3 WHERE uid = $4
               RETURNING uid, email, start_over_date"#,
        )
        .bind(email)
        .bind(phash)
        .bind(start_over_date)
        .bind(uid)
        .fetch_one(tx.as_mut())
        .await?;
        Ok(row)
    }
}

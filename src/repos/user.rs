use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::error::db::DatabaseError;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub uid: Uuid,
    pub email: String,
    pub phash: String,
    pub created_at: DateTime<Utc>,
    pub start_over_date: i16,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPayload {
    pub email: String,
    pub phash: String,
    pub start_over_date: i16,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserPayload {
    pub email: Option<String>,
    pub phash: Option<String>,
    pub start_over_date: Option<i16>,
}

pub struct UserRepo;

impl UserRepo {
    pub async fn list(db: &sqlx::PgPool) -> Result<Vec<User>, DatabaseError> {
        let rows = sqlx::query_as::<_, User>(
            r#"SELECT uid, email, phash, created_at, start_over_date FROM users ORDER BY created_at DESC"#
        )
        .fetch_all(db)
        .await?;
        Ok(rows)
    }

    pub async fn get(db: &sqlx::PgPool, uid: Uuid) -> Result<User, DatabaseError> {
        let row = sqlx::query_as::<_, User>(
            r#"SELECT uid, email, phash, created_at, start_over_date FROM users WHERE uid = $1"#
        )
        .bind(uid)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn create(db: &sqlx::PgPool, payload: CreateUserPayload) -> Result<User, DatabaseError> {
        let uid = Uuid::new_v4();
        let row = sqlx::query_as::<_, User>(
            r#"INSERT INTO users (uid, email, phash, start_over_date)
               VALUES ($1, $2, $3, $4)
               RETURNING uid, email, phash, created_at, start_over_date"#
        )
        .bind(uid)
        .bind(payload.email)
        .bind(payload.phash)
        .bind(payload.start_over_date)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn update(db: &sqlx::PgPool, uid: Uuid, payload: UpdateUserPayload) -> Result<User, DatabaseError> {
        let current = Self::get(db, uid).await?;
        let email = payload.email.unwrap_or(current.email);
        let phash = payload.phash.unwrap_or(current.phash);
        let start_over_date = payload.start_over_date.unwrap_or(current.start_over_date);
        let row = sqlx::query_as::<_, User>(
            r#"UPDATE users SET email = $1, phash = $2, start_over_date = $3 WHERE uid = $4
               RETURNING uid, email, phash, created_at, start_over_date"#
        )
        .bind(email)
        .bind(phash)
        .bind(start_over_date)
        .bind(uid)
        .fetch_one(db)
        .await?;
        Ok(row)
    }

    pub async fn delete(db: &sqlx::PgPool, uid: Uuid) -> Result<(), DatabaseError> {
        sqlx::query("DELETE FROM users WHERE uid = $1").bind(uid)
            .execute(db)
            .await?;
        Ok(())
    }
}

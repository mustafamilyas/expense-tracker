
use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};
use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::info;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::{error::AppError, types::AppState};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub uid: Uuid,
    pub email: String,
    pub phash: String,
    pub created_at: DateTime<Utc>,
    pub start_over_date: i16,
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/users", axum::routing::get(list_users))
        .route("/users/{uid}", axum::routing::get(get_user).put(update_user).delete(delete_user))
        .route("/register", axum::routing::post(create_user))
        .route("/login", axum::routing::post(login_user))
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct UserRead {
    pub uid: Uuid,
    pub email: String,
    pub start_over_date: i16,
}

async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<UserRead>>, AppError> {
    let db_pool = &state.db_pool;
    let rows = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date
        FROM users
        "#
    )
    .fetch_all(db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPayload {
    email: String,
    password: String,
    start_over_date: i16,
}

async fn create_user(State(state): State<AppState>, Json(payload): Json<CreateUserPayload>) -> Result<Json<UserRead>, AppError> {
    let db_pool = &state.db_pool;
    let salt = SaltString::generate(&mut OsRng);
    let phash = argon2::Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .to_string();
    let uid = Uuid::new_v4();
        
    let _res = sqlx::query(
        r#"
        INSERT INTO users (uid, email, phash, start_over_date)
        VALUES ($1, $2, $3, $4)
        "#
    )
    .bind(uid)
    .bind(&payload.email)
    .bind(&phash)
    .bind(payload.start_over_date)
    .execute(db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;
    Ok(Json(UserRead {
        uid,
        email: payload.email.clone(),
        start_over_date: payload.start_over_date,
    }))
}

async fn get_user(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<Json<UserRead>, AppError> {
    info!("Fetching user with uid: {}", uid);
    let user = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date
        FROM users
        WHERE uid = $1
        "#
    )
    .bind(uid)
    .fetch_optional(&state.db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;

    match user {
        Some(u) => Ok(Json(u)),
        None => Err(AppError::NotFound),
    }
}

#[derive(Deserialize)]
pub struct UpdateUserPayload {
    email: Option<String>,
    password: Option<String>,
    start_over_date: Option<i16>,
}

async fn update_user(State(state): State<AppState>, Path(uid): Path<Uuid>, Json(payload): Json<UpdateUserPayload>) -> Result<Json<UserRead>, AppError> {
    info!("Updating user with uid: {}", uid);
    let user: User = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date, phash, created_at
        FROM users
        WHERE uid = $1
        "#
    )
    .bind(uid)
    .fetch_one(&state.db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;

    let email = payload.email.unwrap_or(user.email);
    let start_over_date = payload.start_over_date.unwrap_or(user.start_over_date);
    let phash = if let Some(password) = payload.password {
        let salt = SaltString::generate(&mut OsRng);
        argon2::Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))    ?
            .to_string()
    } else {
        user.phash
    };

    let _res = sqlx::query(
        r#"
        UPDATE users
        SET email = $1, phash = $2, start_over_date = $3, updated_at = now()
        WHERE uid = $4
        "#
    )
    .bind(&email)
    .bind(&phash)
    .bind(start_over_date)
    .bind(uid)
    .execute(&state.db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;
    let updated_user = UserRead {
        uid,
        email,
        start_over_date,
    };


    Ok(Json(updated_user))
}

async fn delete_user(State(state): State<AppState>, Path(uid): Path<Uuid>) -> Result<(), AppError> {
    info!("Deleting user with uid: {}", uid);
    let _ = sqlx::query(
        r#"
        DELETE FROM users
        WHERE uid = $1
        "#
    )
    .bind(uid)
    .execute(&state.db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;
    Ok(())
}

#[derive(Deserialize)]
pub struct LoginUserPayload {
    email: String,
    password: String,
}

async fn login_user(State(state): State<AppState>, Json(payload): Json<LoginUserPayload>) -> Result<Json<UserRead>, AppError> {
    let user: User = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date, phash, created_at
        FROM users
        WHERE email = $1
        "#
    )
    .bind(&payload.email)
    .fetch_one(&state.db_pool)
    .await.map_err(
        |e| AppError::Internal(anyhow::anyhow!(e))
    )?;

    info!("User found: {:?}", user);

    let phash = PasswordHash::new(&user.phash)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    if !argon2::Argon2::default().verify_password(payload.password.as_bytes(), &phash).is_ok() {
        return Err(AppError::Unauthorized("Invalid email or password".into()));
    }

    Ok(Json(UserRead {
        uid: user.uid,
        email: user.email,
        start_over_date: user.start_over_date,
    }))
}







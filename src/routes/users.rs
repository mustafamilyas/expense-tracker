use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tracing::info;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{error::AppError, repos::user::User, types::AppState};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/users", axum::routing::get(list_users))
        .route(
            "/users/{uid}",
            axum::routing::get(get_user)
                .put(update_user)
                .delete(delete_user),
        )
        .route("/register", axum::routing::post(create_user))
        .route("/login", axum::routing::post(login_user))
}

#[derive(Debug, Serialize, Deserialize, FromRow, ToSchema)]
pub struct UserRead {
    pub uid: Uuid,
    pub email: String,
    pub start_over_date: i16,
}

#[utoipa::path(get, path = "/users", responses((status = 200, body = [UserRead])), tag = "Users")]
pub async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<UserRead>>, AppError> {
    let db_pool = &state.db_pool;
    let rows = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date
        FROM users
        "#,
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserPayload {
    pub email: String,
    pub password: String,
    pub start_over_date: i16,
}

#[utoipa::path(post, path = "/register", request_body = CreateUserPayload, responses((status = 200, body = UserRead)), tag = "Users")]
pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<UserRead>, AppError> {
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
        "#,
    )
    .bind(uid)
    .bind(&payload.email)
    .bind(&phash)
    .bind(payload.start_over_date)
    .execute(db_pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    // Create default expense groups for the new user

    Ok(Json(UserRead {
        uid,
        email: payload.email.clone(),
        start_over_date: payload.start_over_date,
    }))
}

#[utoipa::path(get, path = "/users/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = UserRead), (status = 404, description = "Not found")), tag = "Users")]
pub async fn get_user(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<UserRead>, AppError> {
    info!("Fetching user with uid: {}", uid);
    let user = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date
        FROM users
        WHERE uid = $1
        "#,
    )
    .bind(uid)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    match user {
        Some(u) => Ok(Json(u)),
        None => Err(AppError::NotFound),
    }
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateUserPayload {
    pub email: Option<String>,
    pub password: Option<String>,
    pub start_over_date: Option<i16>,
}

#[utoipa::path(put, path = "/users/{uid}", params(("uid" = Uuid, Path)), request_body = UpdateUserPayload, responses((status = 200, body = UserRead)), tag = "Users")]
pub async fn update_user(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateUserPayload>,
) -> Result<Json<UserRead>, AppError> {
    info!("Updating user with uid: {}", uid);
    let user: User = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date, phash, created_at
        FROM users
        WHERE uid = $1
        "#,
    )
    .bind(uid)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    let email = payload.email.unwrap_or(user.email);
    let start_over_date = payload.start_over_date.unwrap_or(user.start_over_date);
    let phash = if let Some(password) = payload.password {
        let salt = SaltString::generate(&mut OsRng);
        argon2::Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
            .to_string()
    } else {
        user.phash
    };

    let _res = sqlx::query(
        r#"
        UPDATE users
        SET email = $1, phash = $2, start_over_date = $3
        WHERE uid = $4
        "#,
    )
    .bind(&email)
    .bind(&phash)
    .bind(start_over_date)
    .bind(uid)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let updated_user = UserRead {
        uid,
        email,
        start_over_date,
    };

    Ok(Json(updated_user))
}

#[derive(Serialize, ToSchema)]
pub struct DeleteResponse {
    success: bool,
}

#[utoipa::path(delete, path = "/users/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Users")]
pub async fn delete_user(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<DeleteResponse>, AppError> {
    info!("Deleting user with uid: {}", uid);
    let _ = sqlx::query(
        r#"
        DELETE FROM users
        WHERE uid = $1
        "#,
    )
    .bind(uid)
    .execute(&state.db_pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(DeleteResponse { success: true }))
}

#[derive(Deserialize, ToSchema)]
pub struct LoginUserPayload {
    pub email: String,
    pub password: String,
}

#[utoipa::path(post, path = "/login", request_body = LoginUserPayload, responses((status = 200, body = UserRead), (status = 401, description = "Unauthorized")), tag = "Users")]
pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<UserRead>, AppError> {
    let user: User = sqlx::query_as(
        r#"
        SELECT uid, email, start_over_date, phash, created_at
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(&payload.email)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    info!("User found: {:?}", user);

    let phash =
        PasswordHash::new(&user.phash).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    if !Argon2::default()
        .verify_password(payload.password.as_bytes(), &phash)
        .is_ok()
    {
        return Err(AppError::Unauthorized("Invalid email or password".into()));
    }

    Ok(Json(UserRead {
        uid: user.uid,
        email: user.email,
        start_over_date: user.start_over_date,
    }))
}

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::AppError,
    repos::{
        expense_group::{CreateExpenseGroupPayload, ExpenseGroupRepo},
        user::{CreateUserDbPayload, UserRead, UserRepo},
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/users", axum::routing::get(list_users))
        .route(
            "/users/{uid}",
            axum::routing::get(get_user).put(update_user),
        )
        .route("/auth/register", axum::routing::post(create_user))
        .route("/auth/login", axum::routing::post(login_user))
}

#[utoipa::path(get, path = "/users", responses((status = 200, body = [UserRead])), tag = "Users")]
pub async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<UserRead>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = UserRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserPayload {
    pub email: String,
    pub password: String,
    pub start_over_date: i16,
}

#[utoipa::path(post, path = "/auth/register", request_body = CreateUserPayload, responses((status = 200, body = UserRead)), tag = "Users")]
pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<UserRead>, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let phash = argon2::Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .to_string();

    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let user = UserRepo::create(
        &mut tx,
        CreateUserDbPayload {
            email: payload.email.clone(),
            phash,
            start_over_date: payload.start_over_date,
        },
    )
    .await?;

    let _ = ExpenseGroupRepo::create(
        &mut tx,
        CreateExpenseGroupPayload {
            name: "Default".to_string(),
            owner: user.uid,
        },
    )
    .await?;

    tx.commit().await.map_err(|e| AppError::from(e))?;

    Ok(Json(UserRead {
        uid: user.uid,
        email: payload.email.clone(),
        start_over_date: payload.start_over_date,
    }))
}

#[utoipa::path(get, path = "/users/{uid}", params(("uid" = Uuid, Path)), responses((status = 200, body = UserRead), (status = 404, description = "Not found")), tag = "Users")]
pub async fn get_user(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<UserRead>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let user = UserRepo::get(&mut tx, uid).await.ok();
    tx.commit().await.map_err(|e| AppError::from(e))?;

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
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let new_phash = match &payload.password {
        Some(pw) => {
            let salt = SaltString::generate(&mut OsRng);
            Some(
                argon2::Argon2::default()
                    .hash_password(pw.as_bytes(), &salt)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
                    .to_string(),
            )
        }
        None => None,
    };
    let updated_user = UserRepo::update(
        &mut tx,
        uid,
        crate::repos::user::UpdateUserDbPayload {
            email: payload.email,
            phash: new_phash,
            start_over_date: payload.start_over_date,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(updated_user))
}

#[derive(Deserialize, ToSchema)]
pub struct LoginUserPayload {
    pub email: String,
    pub password: String,
}

#[utoipa::path(post, path = "/auth/login", request_body = LoginUserPayload, responses((status = 200, body = UserRead), (status = 401, description = "Unauthorized")), tag = "Users")]
pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<UserRead>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let user = UserRepo::get_by_email(&mut tx, &payload.email)
        .await
        .map_err(|_| AppError::Unauthorized("Invalid email or password".into()))?;
    tx.commit().await.map_err(|e| AppError::from(e))?;

    let phash =
        PasswordHash::new(&user.phash).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    if !Argon2::default()
        .verify_password(payload.password.as_bytes(), &phash)
        .is_ok()
    {
        return Err(AppError::Unauthorized("Invalid email or password".into()));
    }

    // TODO: Generate and return a JWT or session token here for authenticated sessions
    Ok(Json(UserRead {
        uid: user.uid,
        email: user.email,
        start_over_date: user.start_over_date,
    }))
}

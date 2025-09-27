use core::convert::From;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::{
    extract::{Path, State}, Extension, Json
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::{
    auth::AuthContext, error::AppError, repos::{
        expense_group::{CreateExpenseGroupDbPayload, ExpenseGroupRepo}, subscription::{CreateSubscriptionDbPayload, SubscriptionRepo}, user::{CreateUserDbPayload, UserRead, UserRepo}
    }, types::{AppState, SubscriptionTier}
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        // .route("/users", axum::routing::get(list_users))
        .route(
            "/users/{uid}",
            axum::routing::put(update_user),
        )
        .route("/users/me", axum::routing::get(get_me)) // alias for get_user
        .route("/auth/register", axum::routing::post(create_user))
        .route("/auth/login", axum::routing::post(login_user))
    
}

// TODO: restrict to admin users only
#[utoipa::path(
    get, 
    path = "/users", 
    responses((status = 200, body = [UserRead])), 
    tag = "Users", 
    operation_id = "listUsers", 
    security(("bearerAuth" = []))
)]
pub async fn list_users(State(state): State<AppState>) -> Result<Json<Vec<UserRead>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for listing users"))?;
    let res = UserRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for listing users"))?;
    Ok(Json(res))
}

#[derive(Debug, Deserialize, serde::Serialize, ToSchema, Validate)]
pub struct CreateUserPayload {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    pub password: String,
    #[validate(range(min = 1, max = 28))]
    pub start_over_date: i16,
}

#[utoipa::path(post, path = "/auth/register", request_body = CreateUserPayload, responses((status = 200, body = UserRead)), tag = "Users", operation_id = "createUser")]
pub async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserPayload>,
) -> Result<Json<UserRead>, AppError> {
    payload.validate()?;
    let salt = SaltString::generate(&mut OsRng);
    let phash = argon2::Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?
        .to_string();

    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for creating user"))?;
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
        CreateExpenseGroupDbPayload {
            name: "Default".to_string(),
            owner: user.uid,
        },
    )
    .await?;

    // let _ = SubscriptionRepo::create(
    //     &mut tx,
    //     CreateSubscriptionDbPayload {
    //         user_uid: user.uid,
    //         tier: SubscriptionTier::Personal,
    //         status: Some("active".to_string()),
    //         current_period_start: None,
    //         current_period_end: None,
    //     },
    // ).await?;

    // For demo purposes, every new user gets a personal subscription for three months
    let start = chrono::Utc::now();
    // TODO: End exactly 3 months later on the same day, if that day does not exist, use the last day of that month
    // For example, if start is Jan 31, end should be Apr 30
    // For now, just add 90 days
    let end = start + chrono::Duration::days(90);
    let _ = SubscriptionRepo::create(
        &mut tx,
        CreateSubscriptionDbPayload {
            user_uid: user.uid,
            tier: SubscriptionTier::Personal,
            status: Some("active".to_string()),
            current_period_start: Some(start),
            current_period_end: Some(end),
        },
    ).await?;


    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for creating user"))?;

    Ok(Json(UserRead {
        uid: user.uid,
        email: payload.email.clone(),
        start_over_date: payload.start_over_date,
    }))
}

#[utoipa::path(
    get, 
    path = "/users/me", 
    responses((status = 200, body = UserRead), (status = 404, description = "Not found")), 
    tag = "Users", 
    operation_id = "getMe",
    security(
        ("bearerAuth" = [])
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<UserRead>, AppError> {
    let user_uid = auth.user_uid;
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for getting user"))?;
    let user = UserRepo::get(&mut tx, user_uid).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for getting user"))?;

    Ok(Json(user))
}

// TODO: restrict to admin users or the user themselves
#[derive(Deserialize, ToSchema, Validate)]
pub struct UpdateUserPayload {
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 8))]
    pub password: Option<String>,
    #[validate(range(min = 1, max = 28))]
    pub start_over_date: Option<i16>,
}

#[utoipa::path(put, path = "/users/{uid}", params(("uid" = Uuid, Path)), request_body = UpdateUserPayload, responses((status = 200, body = UserRead)), tag = "Users", operation_id = "updateUser", security(("bearerAuth" = [])))]
pub async fn update_user(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
    Json(payload): Json<UpdateUserPayload>,
) -> Result<Json<UserRead>, AppError> {
    payload.validate()?;
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for updating user"))?;
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
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for updating user"))?;
    Ok(Json(updated_user))
}

#[derive(Deserialize, serde::Serialize, ToSchema)]
pub struct LoginUserPayload {
    pub email: String,
    pub password: String,
}

#[derive(serde::Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserRead,
}

#[utoipa::path(post, path = "/auth/login", request_body = LoginUserPayload, responses((status = 200, body = LoginResponse), (status = 401, description = "Unauthorized")), tag = "Users", operation_id = "loginUser")]
pub async fn login_user(
    State(state): State<AppState>,
    Json(payload): Json<LoginUserPayload>,
) -> Result<Json<LoginResponse>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for user login"))?;
    let user = UserRepo::get_by_email(&mut tx, &payload.email)
        .await
        .map_err(|_| AppError::Unauthorized("Invalid email or password".into()))?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for user login"))?;

    let phash =
        PasswordHash::new(&user.phash).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;

    if !Argon2::default()
        .verify_password(payload.password.as_bytes(), &phash)
        .is_ok()
    {
        return Err(AppError::Unauthorized("Invalid email or password".into()));
    }

    // Issue JWT for web clients
    let token = crate::auth::encode_web_jwt(user.uid, &state.jwt_secret, 60 * 60 * 24 * 7)
        .map_err(AppError::Internal)?;

    Ok(Json(LoginResponse {
        token,
        user: UserRead {
            uid: user.uid,
            email: user.email,
            start_over_date: user.start_over_date,
        },
    }))
}

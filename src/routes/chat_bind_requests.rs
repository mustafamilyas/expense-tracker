use axum::{
    Extension, Json,
    extract::{Path, State},
};
use serde::Deserialize;
use utoipa::{ ToSchema};
use uuid::Uuid;

use crate::{
    error::AppError,
    repos::chat_bind_request::{
        ChatBindRequest, ChatBindRequestRepo, CreateChatBindRequestDbPayload,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/chat-bind-requests", axum::routing::post(create))
        .route("/chat-bind-requests/{id}", axum::routing::get(get))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateChatBindRequestPayload {
    pub platform: String,
    pub p_uid: String,
    pub nonce: String,
    pub user_uid: Option<Uuid>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[utoipa::path(post, path = "/chat-bind-requests", request_body = CreateChatBindRequestPayload, responses((status = 200, body = ChatBindRequest)), tag = "Chat Bind Requests", operation_id = "createChatBindRequest", security(("bearerAuth" = [])))]
pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateChatBindRequestPayload>,
) -> Result<Json<ChatBindRequest>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::from_sqlx_error(e, "beginning transaction for creating chat bind request")
    })?;
    let created = ChatBindRequestRepo::create(
        &mut tx,
        CreateChatBindRequestDbPayload {
            platform: payload.platform,
            p_uid: payload.p_uid,
            nonce: payload.nonce,
            user_uid: payload.user_uid,
            expires_at: payload.expires_at,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| {
        AppError::from_sqlx_error(e, "committing transaction for creating chat bind request")
    })?;
    Ok(Json(created))
}

#[utoipa::path(
    get, 
    path = "/chat-bind-requests/{uid}", 
    responses((status = 200, body = ChatBindRequest)), 
    params(("uid" = Uuid, Path, description = "The UUID of the chat bind request to retrieve")),
    tag = "Chat Bind Requests", operation_id = "getChatBindRequest", security(("bearerAuth" = [])))]
pub async fn get(
    State(state): State<AppState>,
    Path(uid): Path<Uuid>,
) -> Result<Json<ChatBindRequest>, AppError> {
    // TODO: restrict security
    let mut tx = state.db_pool.begin().await.map_err(|e| {
        AppError::from_sqlx_error(e, "beginning transaction for getting chat bind request")
    })?;
    let res = ChatBindRequestRepo::get(&mut tx, uid).await?;
    tx.commit().await.map_err(|e| {
        AppError::from_sqlx_error(e, "committing transaction for getting chat bind request")
    })?;
    Ok(Json(res))
}

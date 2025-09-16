use axum::{
    Json,
    extract::{Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::AppError,
    repos::chat_bind_request::{
        ChatBindRequest, ChatBindRequestRepo, CreateChatBindRequestDbPayload,
        UpdateChatBindRequestDbPayload,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{id}", axum::routing::get(get).put(update).delete(delete_))
}

#[utoipa::path(get, path = "/chat-bind-requests", responses((status = 200, body = [ChatBindRequest])), tag = "Chat Bind Requests", operation_id = "listChatBindRequests", security(("bearerAuth" = [])))]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ChatBindRequest>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = ChatBindRequestRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/chat-bind-requests/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = ChatBindRequest)), tag = "Chat Bind Requests", operation_id = "getChatBindRequest", security(("bearerAuth" = [])))]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ChatBindRequest>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = ChatBindRequestRepo::get(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
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
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
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
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateChatBindRequestPayload {
    pub user_uid: Option<Option<Uuid>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[utoipa::path(put, path = "/chat-bind-requests/{id}", params(("id" = Uuid, Path)), request_body = UpdateChatBindRequestPayload, responses((status = 200, body = ChatBindRequest)), tag = "Chat Bind Requests", operation_id = "updateChatBindRequest", security(("bearerAuth" = [])))]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateChatBindRequestPayload>,
) -> Result<Json<ChatBindRequest>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let updated = ChatBindRequestRepo::update(
        &mut tx,
        id,
        UpdateChatBindRequestDbPayload {
            user_uid: payload.user_uid,
            expires_at: payload.expires_at,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/chat-bind-requests/{id}", params(("id" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Chat Bind Requests", operation_id = "deleteChatBindRequest", security(("bearerAuth" = [])))]
pub async fn delete_(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    ChatBindRequestRepo::delete(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(())
}

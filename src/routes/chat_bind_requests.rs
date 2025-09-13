use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{error::app::AppError, repos::chat_bind_request::{ChatBindRequest, ChatBindRequestRepo, CreateChatBindRequestPayload, UpdateChatBindRequestPayload}, types::AppState};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{id}", axum::routing::get(get).put(update).delete(delete_))
}

#[utoipa::path(get, path = "/chat-bind-requests", responses((status = 200, body = [ChatBindRequest])), tag = "Chat Bind Requests")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ChatBindRequest>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = ChatBindRequestRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/chat-bind-requests/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = ChatBindRequest)), tag = "Chat Bind Requests")]
pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<ChatBindRequest>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = ChatBindRequestRepo::get(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload { pub platform: String, pub p_uid: String, pub nonce: String, pub user_uid: Option<Uuid>, pub expires_at: chrono::DateTime<chrono::Utc> }

#[utoipa::path(post, path = "/chat-bind-requests", request_body = CreatePayload, responses((status = 200, body = ChatBindRequest)), tag = "Chat Bind Requests")]
pub async fn create(State(state): State<AppState>, Json(payload): Json<CreatePayload>) -> Result<Json<ChatBindRequest>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let created = ChatBindRequestRepo::create(&mut tx, CreateChatBindRequestPayload {
        platform: payload.platform, p_uid: payload.p_uid, nonce: payload.nonce, user_uid: payload.user_uid, expires_at: payload.expires_at
    }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload { pub user_uid: Option<Option<Uuid>>, pub expires_at: Option<chrono::DateTime<chrono::Utc>> }

#[utoipa::path(put, path = "/chat-bind-requests/{id}", params(("id" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = ChatBindRequest)), tag = "Chat Bind Requests")]
pub async fn update(State(state): State<AppState>, Path(id): Path<Uuid>, Json(payload): Json<UpdatePayload>) -> Result<Json<ChatBindRequest>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let updated = ChatBindRequestRepo::update(&mut tx, id, UpdateChatBindRequestPayload { user_uid: payload.user_uid, expires_at: payload.expires_at }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/chat-bind-requests/{id}", params(("id" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Chat Bind Requests")]
pub async fn delete_(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    ChatBindRequestRepo::delete(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(())
}

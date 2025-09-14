use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{error::AppError, repos::chat_binding::{ChatBinding, ChatBindingRepo, CreateChatBindingPayload, UpdateChatBindingPayload}, types::AppState};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{id}", axum::routing::get(get).put(update).delete(delete_))
}

#[utoipa::path(get, path = "/chat-bindings", responses((status = 200, body = [ChatBinding])), tag = "Chat Bindings")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ChatBinding>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = ChatBindingRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/chat-bindings/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = ChatBinding)), tag = "Chat Bindings")]
pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<ChatBinding>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let res = ChatBindingRepo::get(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload { pub group_uid: Uuid, pub platform: String, pub p_uid: String, pub status: Option<String>, pub bound_by: Uuid }

#[utoipa::path(post, path = "/chat-bindings", request_body = CreatePayload, responses((status = 200, body = ChatBinding)), tag = "Chat Bindings")]
pub async fn create(State(state): State<AppState>, Json(payload): Json<CreatePayload>) -> Result<Json<ChatBinding>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let created = ChatBindingRepo::create(&mut tx, CreateChatBindingPayload {
        group_uid: payload.group_uid,
        platform: payload.platform,
        p_uid: payload.p_uid,
        status: payload.status,
        bound_by: payload.bound_by,
    }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload { pub status: Option<String>, pub revoked_at: Option<Option<chrono::DateTime<chrono::Utc>>> }

#[utoipa::path(put, path = "/chat-bindings/{id}", params(("id" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = ChatBinding)), tag = "Chat Bindings")]
pub async fn update(State(state): State<AppState>, Path(id): Path<Uuid>, Json(payload): Json<UpdatePayload>) -> Result<Json<ChatBinding>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    let updated = ChatBindingRepo::update(&mut tx, id, UpdateChatBindingPayload { status: payload.status, revoked_at: payload.revoked_at }).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/chat-bindings/{id}", params(("id" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Chat Bindings")]
pub async fn delete_(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    ChatBindingRepo::delete(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    Ok(())
}

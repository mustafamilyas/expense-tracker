use axum::{Json, extract::State};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    error::AppError,
    repos::chat_bind_request::{
        ChatBindRequest, ChatBindRequestRepo, CreateChatBindRequestDbPayload,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/chat-bind-requests", axum::routing::post(create))
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

use axum::{
    Json,
    extract::{Extension, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, group_guard::group_guard},
    error::AppError,
    repos::{
        chat_bind_request::ChatBindRequestRepo,
        chat_binding::{ChatBinding, ChatBindingRepo, CreateChatBindingDbPayload},
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/chat-bindings/accept", axum::routing::post(accept))
}

/*
Workflow:
1) User types `/sign-in` in chat.
2) Server creates a `ChatBindRequest { platform, p_uid, nonce, expires_at }` and replies with a URL (contains id + nonce) to open in the web dashboard.
3) User logs in to web; server verifies request id+nonce and expiry; user selects expense group to bind.
4) Server creates `ChatBinding { group_uid, platform, p_uid, status='active', bound_by=user_uid }`, marks the request used, and sends a welcome message in chat.

accept should handle step 3 and 4.
 */

#[derive(Deserialize, ToSchema)]
pub struct AcceptChatBindingPayload {
    pub request_id: Uuid,
    pub nonce: String,
    pub group_uid: Uuid,
}

#[utoipa::path(post, path = "/chat-bindings/accept", request_body = AcceptChatBindingPayload, responses((status = 200, body = ChatBinding)), tag = "Chat Bindings", operation_id = "acceptChatBinding", security(("bearerAuth" = [])))]
pub async fn accept(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<AcceptChatBindingPayload>,
) -> Result<Json<ChatBinding>, AppError> {
    group_guard(&auth, payload.group_uid, &state.db_pool).await?;

    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for accepting chat binding"))?;
    let chat_bind_request = ChatBindRequestRepo::get(&mut tx, payload.request_id).await?;
    // TODO: proper nonce handling (e.g. one-time use)
    if chat_bind_request.nonce != payload.nonce {
        return Err(AppError::BadRequest("Invalid nonce".into()));
    }
    if chat_bind_request.expires_at < chrono::Utc::now() {
        ChatBindRequestRepo::delete(&mut tx, payload.request_id).await?;
        tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for expired chat bind request"))?;
        return Err(AppError::BadRequest("Chat bind request expired".into()));
    }
    let created = ChatBindingRepo::create(
        &mut tx,
        CreateChatBindingDbPayload {
            group_uid: payload.group_uid,
            platform: chat_bind_request.platform.clone(),
            p_uid: chat_bind_request.p_uid.clone(),
            status: Some("active".into()),
            bound_by: auth.user_uid,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for creating chat binding"))?;

    // Send welcome message to the chat
    if let Some(messenger_manager) = &state.messenger_manager {
        let welcome_message = "🎉 Welcome! Your expense tracker is now ready to use!\n\n\
            📝 Available commands:\n\
            • /expense - Add new expenses\n\
            • /expense-edit - Edit existing expenses\n\
            • /report - View monthly summary\n\
            • /history - View expense history\n\
            • /budget - View budget overview\n\
            • /category - Manage categories\n\
            • /command - Show all commands\n\n\
            💡 Start by adding your first expense with /expense!";

        if let Err(e) = messenger_manager.send_message(&created.platform, &created.p_uid, welcome_message).await {
            tracing::error!("Failed to send welcome message: {:?}", e);
        }
    }

    Ok(Json(created))
}

// #[utoipa::path(get, path = "/chat-bindings", responses((status = 200, body = [ChatBinding])), tag = "Chat Bindings", operation_id = "listChatBindings", security(("bearerAuth" = [])))]
// pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<ChatBinding>>, AppError> {
//     let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     let res = ChatBindingRepo::list(&mut tx).await?;
//     tx.commit().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     Ok(Json(res))
// }

// #[utoipa::path(get, path = "/chat-bindings/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = ChatBinding)), tag = "Chat Bindings", operation_id = "getChatBinding", security(("bearerAuth" = [])))]
// pub async fn get(
//     State(state): State<AppState>,
//     Path(id): Path<Uuid>,
// ) -> Result<Json<ChatBinding>, AppError> {
//     let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     let res = ChatBindingRepo::get(&mut tx, id).await?;
//     tx.commit().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     Ok(Json(res))
// }

// #[derive(Deserialize, ToSchema)]
// pub struct CreateChatBindingPayload {
//     pub group_uid: Uuid,
//     pub platform: String,
//     pub p_uid: String,
//     pub status: Option<String>,
//     pub bound_by: Uuid,
// }

// #[utoipa::path(post, path = "/chat-bindings", request_body = CreateChatBindingPayload, responses((status = 200, body = ChatBinding)), tag = "Chat Bindings", operation_id = "createChatBinding", security(("bearerAuth" = [])))]
// pub async fn create(
//     State(state): State<AppState>,
//     Extension(auth): Extension<AuthContext>,
//     Json(payload): Json<CreateChatBindingPayload>,
// ) -> Result<Json<ChatBinding>, AppError> {
//     if matches!(auth.source, AuthSource::Chat) && auth.group_uid != Some(payload.group_uid) {
//         return Err(AppError::Unauthorized("Group scope mismatch".into()));
//     }
//     let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     let created = ChatBindingRepo::create(
//         &mut tx,
//         CreateChatBindingDbPayload {
//             group_uid: payload.group_uid,
//             platform: payload.platform,
//             p_uid: payload.p_uid,
//             status: payload.status,
//             bound_by: payload.bound_by,
//         },
//     )
//     .await?;
//     tx.commit().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     Ok(Json(created))
// }

// #[derive(Deserialize, ToSchema)]
// pub struct UpdateChatBindingPayload {
//     pub status: Option<String>,
//     pub revoked_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
// }

// #[utoipa::path(put, path = "/chat-bindings/{id}", params(("id" = Uuid, Path)), request_body = UpdateChatBindingPayload, responses((status = 200, body = ChatBinding)), tag = "Chat Bindings", operation_id = "updateChatBinding", security(("bearerAuth" = [])))]
// pub async fn update(
//     State(state): State<AppState>,
//     Path(id): Path<Uuid>,
//     Json(payload): Json<UpdateChatBindingPayload>,
// ) -> Result<Json<ChatBinding>, AppError> {
//     let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     let updated = ChatBindingRepo::update(
//         &mut tx,
//         id,
//         UpdateChatBindingDbPayload {
//             status: payload.status,
//             revoked_at: payload.revoked_at,
//         },
//     )
//     .await?;
//     tx.commit().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     Ok(Json(updated))
// }

// #[utoipa::path(delete, path = "/chat-bindings/{id}", params(("id" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Chat Bindings", operation_id = "deleteChatBinding", security(("bearerAuth" = [])))]
// pub async fn delete_(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<(), AppError> {
//     let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     ChatBindingRepo::delete(&mut tx, id).await?;
//     tx.commit().await.map_err(|e| AppError::from_sqlx_error(e))?;
//     Ok(())
// }

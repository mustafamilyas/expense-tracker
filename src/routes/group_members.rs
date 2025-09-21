use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, AuthSource},
    error::AppError,
    repos::expense_group_member::{
        CreateGroupMemberDbPayload, GroupMember, GroupMemberRepo, UpdateGroupMemberDbPayload,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/group-members", axum::routing::get(list).post(create))
        .route(
            "/group-members/{id}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

/*
Before activating these routes, make sure to:
1. What can users do with group members? (e.g., only admins can add/remove members)
2. Ensure proper authentication and authorization checks are in place.
3. What can group members see and do? (e.g., can they see other members, their roles, etc.)
 */

#[utoipa::path(get, path = "/group-members", responses((status = 200, body = [GroupMember])), tag = "Group Members", operation_id = "listGroupMembers", security(("bearerAuth" = [])))]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<GroupMember>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for listing group members"))?;
    let res = GroupMemberRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for listing group members"))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/group-members/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = GroupMember)), tag = "Group Members", operation_id = "getGroupMember", security(("bearerAuth" = [])))]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GroupMember>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for getting group member"))?;
    let res = GroupMemberRepo::get(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for getting group member"))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateGroupMemberPayload {
    pub group_uid: Uuid,
    pub user_uid: Uuid,
    pub role: String,
}

#[utoipa::path(post, path = "/group-members", request_body = CreateGroupMemberPayload, responses((status = 200, body = GroupMember)), tag = "Group Members", operation_id = "createGroupMember", security(("bearerAuth" = [])))]
pub async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateGroupMemberPayload>,
) -> Result<Json<GroupMember>, AppError> {
    if matches!(auth.source, AuthSource::Chat) && auth.group_uid != Some(payload.group_uid) {
        return Err(AppError::Unauthorized("Group scope mismatch".into()));
    }
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for creating group member"))?;
    let created = GroupMemberRepo::create(
        &mut tx,
        CreateGroupMemberDbPayload {
            group_uid: payload.group_uid,
            user_uid: payload.user_uid,
            role: payload.role,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for creating group member"))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateGroupMemberPayload {
    pub role: Option<String>,
}

#[utoipa::path(put, path = "/group-members/{id}", params(("id" = Uuid, Path)), request_body = UpdateGroupMemberPayload, responses((status = 200, body = GroupMember)), tag = "Group Members", operation_id = "updateGroupMember", security(("bearerAuth" = [])))]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateGroupMemberPayload>,
) -> Result<Json<GroupMember>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for updating group member"))?;
    let updated = GroupMemberRepo::update(
        &mut tx,
        id,
        UpdateGroupMemberDbPayload { role: payload.role },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for updating group member"))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/group-members/{id}", params(("id" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Group Members", operation_id = "deleteGroupMember", security(("bearerAuth" = [])))]
pub async fn delete_(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for deleting group member"))?;
    GroupMemberRepo::delete(&mut tx, id).await?;
    tx.commit().await.map_err(|e| AppError::from_sqlx_error(e, "committing transaction for deleting group member"))?;
    Ok(())
}

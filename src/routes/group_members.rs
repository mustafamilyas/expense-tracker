use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use uuid::Uuid;
use utoipa::ToSchema;

use crate::{error::app::AppError, repos::expense_group_member::{GroupMember, GroupMemberRepo, CreateGroupMemberPayload, UpdateGroupMemberPayload}, types::AppState};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{id}", axum::routing::get(get).put(update).delete(delete_))
}

#[utoipa::path(get, path = "/group-members", responses((status = 200, body = [GroupMember])), tag = "Group Members")]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<GroupMember>>, AppError> {
    Ok(Json(GroupMemberRepo::list(&state.db_pool).await?))
}

#[utoipa::path(get, path = "/group-members/{id}", params(("id" = Uuid, Path)), responses((status = 200, body = GroupMember)), tag = "Group Members")]
pub async fn get(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<GroupMember>, AppError> {
    Ok(Json(GroupMemberRepo::get(&state.db_pool, id).await?))
}

#[derive(Deserialize, ToSchema)]
pub struct CreatePayload { pub group_uid: Uuid, pub user_uid: Uuid, pub role: String }

#[utoipa::path(post, path = "/group-members", request_body = CreatePayload, responses((status = 200, body = GroupMember)), tag = "Group Members")]
pub async fn create(State(state): State<AppState>, Json(payload): Json<CreatePayload>) -> Result<Json<GroupMember>, AppError> {
    let created = GroupMemberRepo::create(&state.db_pool, CreateGroupMemberPayload { group_uid: payload.group_uid, user_uid: payload.user_uid, role: payload.role }).await?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdatePayload { pub role: Option<String> }

#[utoipa::path(put, path = "/group-members/{id}", params(("id" = Uuid, Path)), request_body = UpdatePayload, responses((status = 200, body = GroupMember)), tag = "Group Members")]
pub async fn update(State(state): State<AppState>, Path(id): Path<Uuid>, Json(payload): Json<UpdatePayload>) -> Result<Json<GroupMember>, AppError> {
    let updated = GroupMemberRepo::update(&state.db_pool, id, UpdateGroupMemberPayload { role: payload.role }).await?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/group-members/{id}", params(("id" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Group Members")]
pub async fn delete_(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<(), AppError> {
    GroupMemberRepo::delete(&state.db_pool, id).await?;
    Ok(())
}

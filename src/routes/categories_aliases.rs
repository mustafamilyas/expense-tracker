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
    repos::category_alias::{
        CategoryAlias, CategoryAliasRepo, CreateCategoryAliasDbPayload,
        UpdateCategoryAliasDbPayload,
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route(
            "/{alias_uid}",
            axum::routing::get(get).put(update).delete(delete_),
        )
}

#[utoipa::path(get, path = "/categories-aliases", responses((status = 200, body = [CategoryAlias])), tag = "Category Aliases", operation_id = "listCategoryAliases", security(("bearerAuth" = [])))]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<CategoryAlias>>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = CategoryAliasRepo::list(&mut tx).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[utoipa::path(get, path = "/categories-aliases/{alias_uid}", params(("alias_uid" = Uuid, Path)), responses((status = 200, body = CategoryAlias)), tag = "Category Aliases", operation_id = "getCategoryAlias", security(("bearerAuth" = [])))]
pub async fn get(
    State(state): State<AppState>,
    Path(alias_uid): Path<Uuid>,
) -> Result<Json<CategoryAlias>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let res = CategoryAliasRepo::get(&mut tx, alias_uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(res))
}

#[derive(Deserialize, ToSchema)]
pub struct CreateCategoryAliasPayload {
    pub group_uid: Uuid,
    pub alias: String,
    pub category_uid: Uuid,
}

#[utoipa::path(post, path = "/categories-aliases", request_body = CreateCategoryAliasPayload, responses((status = 200, body = CategoryAlias)), tag = "Category Aliases", operation_id = "createCategoryAlias", security(("bearerAuth" = [])))]
pub async fn create(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(payload): Json<CreateCategoryAliasPayload>,
) -> Result<Json<CategoryAlias>, AppError> {
    if matches!(auth.source, AuthSource::Chat) && auth.group_uid != Some(payload.group_uid) {
        return Err(AppError::Unauthorized("Group scope mismatch".into()));
    }
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let created = CategoryAliasRepo::create(
        &mut tx,
        CreateCategoryAliasDbPayload {
            group_uid: payload.group_uid,
            alias: payload.alias,
            category_uid: payload.category_uid,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(created))
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateCategoryAliasPayload {
    pub alias: Option<String>,
    pub category_uid: Option<Uuid>,
}

#[utoipa::path(put, path = "/categories-aliases/{alias_uid}", params(("alias_uid" = Uuid, Path)), request_body = UpdateCategoryAliasPayload, responses((status = 200, body = CategoryAlias)), tag = "Category Aliases", operation_id = "updateCategoryAlias", security(("bearerAuth" = [])))]
pub async fn update(
    State(state): State<AppState>,
    Path(alias_uid): Path<Uuid>,
    Json(payload): Json<UpdateCategoryAliasPayload>,
) -> Result<Json<CategoryAlias>, AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    let updated = CategoryAliasRepo::update(
        &mut tx,
        alias_uid,
        UpdateCategoryAliasDbPayload {
            alias: payload.alias,
            category_uid: payload.category_uid,
        },
    )
    .await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/categories-aliases/{alias_uid}", params(("alias_uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Category Aliases", operation_id = "deleteCategoryAlias", security(("bearerAuth" = [])))]
pub async fn delete_(
    State(state): State<AppState>,
    Path(alias_uid): Path<Uuid>,
) -> Result<(), AppError> {
    let mut tx = state.db_pool.begin().await.map_err(|e| AppError::from(e))?;
    CategoryAliasRepo::delete(&mut tx, alias_uid).await?;
    tx.commit().await.map_err(|e| AppError::from(e))?;
    Ok(())
}

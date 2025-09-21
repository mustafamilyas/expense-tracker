use axum::{
    Json,
    extract::{Extension, Path, State},
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    auth::{AuthContext, group_guard::group_guard},
    error::AppError,
    repos::{
        category::CategoryRepo,
        category_alias::{
            CategoryAlias, CategoryAliasRepo, CreateCategoryAliasDbPayload,
            UpdateCategoryAliasDbPayload,
        },
    },
    types::AppState,
};

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/categories-aliases", axum::routing::post(create))
        .route(
            "/categories-aliases/category/{category_uid}",
            axum::routing::get(list),
        )
        .route(
            "/categories-aliases/{alias_uid}",
            axum::routing::put(update).delete(delete_),
        )
}

#[utoipa::path(get, path = "/categories-aliases/category/{category_uid}", responses((status = 200, body = [CategoryAlias])), tag = "Category Aliases", operation_id = "listCategoryAliases", security(("bearerAuth" = [])))]
pub async fn list(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(category_uid): Path<Uuid>,
) -> Result<Json<Vec<CategoryAlias>>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for listing category aliases"))?;
    let category = CategoryRepo::get(&mut tx, category_uid).await?;
    group_guard(&auth, category.group_uid, &state.db_pool).await?;
    let res = CategoryAliasRepo::list_by_category(&mut tx, category_uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for listing category aliases"))?;
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
    group_guard(&auth, payload.group_uid, &state.db_pool).await?;
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for creating category alias"))?;
    let created = CategoryAliasRepo::create(
        &mut tx,
        CreateCategoryAliasDbPayload {
            group_uid: payload.group_uid,
            alias: payload.alias,
            category_uid: payload.category_uid,
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for creating category alias"))?;
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
    Extension(auth): Extension<AuthContext>,
    Path(alias_uid): Path<Uuid>,
    Json(payload): Json<UpdateCategoryAliasPayload>,
) -> Result<Json<CategoryAlias>, AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for updating category alias"))?;
    let prev_alias = CategoryAliasRepo::get(&mut tx, alias_uid).await?;
    group_guard(&auth, prev_alias.group_uid, &state.db_pool).await?;
    let updated = CategoryAliasRepo::update(
        &mut tx,
        alias_uid,
        UpdateCategoryAliasDbPayload {
            alias: payload.alias,
            category_uid: payload.category_uid,
        },
    )
    .await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for updating category alias"))?;
    Ok(Json(updated))
}

#[utoipa::path(delete, path = "/categories-aliases/{alias_uid}", params(("alias_uid" = Uuid, Path)), responses((status = 200, description = "Deleted")), tag = "Category Aliases", operation_id = "deleteCategoryAlias", security(("bearerAuth" = [])))]
pub async fn delete_(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(alias_uid): Path<Uuid>,
) -> Result<(), AppError> {
    let mut tx = state
        .db_pool
        .begin()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "beginning transaction for deleting category alias"))?;
    let prev_alias = CategoryAliasRepo::get(&mut tx, alias_uid).await?;
    group_guard(&auth, prev_alias.group_uid, &state.db_pool).await?;
    CategoryAliasRepo::delete(&mut tx, alias_uid).await?;
    tx.commit()
        .await
        .map_err(|e| AppError::from_sqlx_error(e, "committing transaction for deleting category alias"))?;
    Ok(())
}

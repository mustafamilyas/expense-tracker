use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub version: String,
}

#[derive(Serialize, ToSchema)]
pub struct DeleteResponse {
    pub success: bool,
}

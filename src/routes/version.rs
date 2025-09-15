use axum::{extract::State, Json};
use serde::Serialize;
use utoipa::ToSchema;

use crate::types::AppState;

#[derive(Serialize, ToSchema)]
pub struct VersionBody {
    version: String,
}


#[utoipa::path(get, path = "/version", responses((status = 200, body = VersionBody)), tag = "System", operation_id = "getVersion")]
pub async fn version(State(state): State<AppState>) -> Json<VersionBody> {
    Json(VersionBody { version: state.version.clone() })
}

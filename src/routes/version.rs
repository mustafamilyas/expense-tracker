use axum::{extract::State, Json};
use serde::Serialize;

use crate::types::AppState;

#[derive(Serialize)]
pub struct VersionBody {
    version: String,
}


pub async fn version(State(state): State<AppState>) -> Json<VersionBody> {
    Json(VersionBody { version: state.version.clone() })
}
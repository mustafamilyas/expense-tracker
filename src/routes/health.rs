use axum::http::StatusCode;

#[utoipa::path(get, path = "/health", responses((status = 200, description = "OK")), tag = "System")]
pub async fn health() -> StatusCode {
    StatusCode::OK
}

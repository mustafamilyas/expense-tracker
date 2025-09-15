use axum::http::StatusCode;

#[utoipa::path(get, path = "/health", responses((status = 200, description = "OK")), tag = "System", operation_id = "getHealth")]
pub async fn health() -> StatusCode {
    StatusCode::OK
}

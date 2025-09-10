use axum::{http::StatusCode, response::{IntoResponse, Response}};
use thiserror::Error;

use crate::error::db::DatabaseError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
    #[error("unauthorized")]
    Unauthorized(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "not found").into_response(),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            AppError::Internal(err) => {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("internal error: {}", err),
                )
                    .into_response()
            }
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg).into_response(),
        }
    }
}

impl From<DatabaseError> for AppError {
    fn from(err: DatabaseError) -> Self {
        match err {
            DatabaseError::NotFound(_) => AppError::NotFound,
            DatabaseError::ConstraintViolation(msg) => AppError::BadRequest(msg),
            _ => AppError::Internal(err.into()),
        }
    }
}

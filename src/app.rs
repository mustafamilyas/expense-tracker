use axum::{http::StatusCode, routing::{get, post}, Json, Router};
use serde::Deserialize;
use serde::Serialize;

use crate::routes::expense_entry;

pub fn build_router() -> Router {

    Router::new()
        // `GET /` goes to `root`
        .nest("/expense-entries", expense_entry::router())
}
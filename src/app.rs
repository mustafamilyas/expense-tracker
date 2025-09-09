use axum::{routing::get, Router};

use crate::{routes, types::AppState};

pub fn build_router(app_state: AppState) -> Router {

    Router::new()
        .nest("/expense-entries", routes::expense_entry::router())
        .route("/health", get(routes::health::health))
        .route("/version", get(routes::version::version))
        .merge(routes::users::router())
        .with_state(app_state)

}

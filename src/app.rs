use crate::openapi::ApiDoc;
use axum::{Router, routing::get};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{routes, types::AppState};
use axum::middleware;

pub fn build_router(app_state: AppState) -> Router {
    let auth_state = app_state.clone();
    Router::new()
        // .merge("/group-members", routes::group_members::router())
        .route("/health", get(routes::health::health))
        .route("/version", get(routes::version::version))
        .merge(routes::chat_bindings::router())
        .merge(routes::expense_entry::router())
        .merge(routes::chat_bind_requests::router())
        .merge(routes::budgets::router())
        .merge(routes::categories::router())
        .merge(routes::users::router())
        .merge(routes::expense_groups::router())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(
            auth_state,
            crate::auth::auth_middleware,
        ))
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

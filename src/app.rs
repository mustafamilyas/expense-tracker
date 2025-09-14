use crate::openapi::ApiDoc;
use axum::{Router, routing::get};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{routes, types::AppState};

pub fn build_router(app_state: AppState) -> Router {
    Router::new()
        .nest("/expense-entries", routes::expense_entry::router())
        .nest("/categories", routes::categories::router())
        .nest("/categories-aliases", routes::categories_aliases::router())
        .nest("/budgets", routes::budgets::router())
        .nest("/chat-bind-requests", routes::chat_bind_requests::router())
        .nest("/chat-bindings", routes::chat_bindings::router())
        .nest("/group-members", routes::group_members::router())
        .route("/health", get(routes::health::health))
        .route("/version", get(routes::version::version))
        .merge(routes::users::router())
        .merge(routes::expense_groups::router())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .with_state(app_state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

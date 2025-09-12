use axum::{routing::get, Router};
use utoipa_swagger_ui::SwaggerUi;
use utoipa::OpenApi;
use crate::openapi::ApiDoc;

use crate::{routes, types::AppState};

pub fn build_router(app_state: AppState) -> Router {

    Router::new()
        .nest("/expense-entries", routes::expense_entry::router())
        .nest("/expense-groups", routes::expense_groups::router())
        .nest("/categories", routes::categories::router())
        .nest("/categories-aliases", routes::categories_aliases::router())
        .nest("/budgets", routes::budgets::router())
        .nest("/chat-bind-requests", routes::chat_bind_requests::router())
        .nest("/chat-bindings", routes::chat_bindings::router())
        .nest("/group-members", routes::group_members::router())
        .route("/health", get(routes::health::health))
        .route("/version", get(routes::version::version))
        .merge(routes::users::router())
        .merge(SwaggerUi::new("/docs").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .with_state(app_state)

}

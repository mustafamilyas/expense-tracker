use crate::openapi::ApiDoc;
use axum::{Router, routing::get};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{routes, types::AppState};
use axum::middleware;
use tower_http::cors::{Any, CorsLayer};

pub fn build_router(app_state: AppState) -> Router {
    let auth_state = app_state.clone();

    // Configure CORS
    let mut cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_headers(Any);

    // Add allowed origins
    let mut origins = vec![
        "http://localhost:3000".parse().unwrap(),
        "http://localhost:5173".parse().unwrap(), // Vite dev server
    ];

    if let Ok(origin) = app_state.front_end_url.parse() {
        origins.push(origin);
    }

    cors = cors.allow_origin(origins);

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
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

use anyhow::Result;
use expense_tracker::{app, db, types::AppState};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // load secrets
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("JWT_SECRET not set; using insecure default for development");
        "dev-secret-change-me".to_string()
    });
    let chat_relay_secret = env::var("CHAT_RELAY_SECRET").unwrap_or_else(|_| {
        tracing::warn!("CHAT_RELAY_SECRET not set; using insecure default for development");
        "dev-chat-relay-secret".to_string()
    });

    // build our application with a route
    let app = app::build_router(AppState {
        version: "0.1.0".to_string(),
        db_pool: db::make_db_pool("postgres://postgres:postgres@localhost/postgres").await?,
        jwt_secret,
        chat_relay_secret,
    });

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracing::info!("signal received, starting graceful shutdown");
}

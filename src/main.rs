use anyhow::Result;
use expense_tracker::{
    app, db,
    messengers::{MessengerManager, telegram::TelegramMessenger},
    types::AppState,
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // load environment variables from .env file
    dotenv::dotenv()?;

    // load environment variables from .env file
    dotenv::dotenv().ok();

    // load secrets
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("JWT_SECRET not set; using insecure default for development");
        "dev-secret-change-me".to_string()
    });
    let chat_relay_secret = env::var("CHAT_RELAY_SECRET").unwrap_or_else(|_| {
        tracing::warn!("CHAT_RELAY_SECRET not set; using insecure default for development");
        "dev-chat-relay-secret".to_string()
    });

    let telegram_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_else(|_| {
        tracing::warn!("TELEGRAM_BOT_TOKEN not set; Telegram bot will not be started");
        "".to_string()
    });

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        tracing::warn!("DATABASE_URL not set; using default development database");
        "postgres://postgres:postgres@localhost/postgres".to_string()
    });

    let db_pool = db::make_db_pool(&database_url).await?;

    // Initialize messenger manager
    let mut messenger_manager = MessengerManager::new();

    // Add Telegram bot if token is provided
    if !telegram_token.is_empty() {
        let telegram_messenger = TelegramMessenger::new(telegram_token, db_pool.clone());
        messenger_manager.add_messenger(Box::new(telegram_messenger));
    }

    // Start messengers
    if let Err(e) = messenger_manager.start_all().await {
        tracing::error!("Failed to start messengers: {:?}", e);
        return Err(anyhow::anyhow!("Failed to start messengers"));
    }

    // build our application with a route
    let app = app::build_router(AppState {
        version: "0.1.0".to_string(),
        db_pool,
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

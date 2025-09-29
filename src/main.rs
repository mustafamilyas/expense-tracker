use anyhow::Result;
use expense_tracker::{
    app, db,
    lang::Lang,
    messengers::{MessengerManager, telegram::TelegramMessenger},
    reports::ReportScheduler,
    telegram_logger::TelegramLogger,
    types::AppState,
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // initialize tracing
    let config = expense_tracker::config::Config::from_env();
    let lang = Lang::from_json("id");

    let registry = tracing_subscriber::registry();

    if let (Some(token), Some(chat_id)) = (
        config.telegram_log_token.clone(),
        config.telegram_log_chat_id,
    ) {
        let telegram_logger = TelegramLogger::new(token, chat_id);
        registry
            .with(telegram_logger)
            .with(tracing_subscriber::fmt::layer())
            .init();
    } else {
        registry.with(tracing_subscriber::fmt::layer()).init();
    }

    let db_pool = db::make_db_pool(&config.database_url).await?;

    // Initialize messenger manager
    let mut messenger_manager = MessengerManager::new();

    // Add Telegram bot if token is provided
    if !config.telegram_bot_token.is_empty() {
        let telegram_messenger = TelegramMessenger::new(&config, db_pool.clone());
        messenger_manager.add_messenger(Box::new(telegram_messenger));
    }

    // Create Arc for messenger manager
    let messenger_manager_arc = Arc::new(messenger_manager);

    // Start messengers
    if let Err(e) = messenger_manager_arc.start_all().await {
        tracing::error!("Failed to start messengers: {:?}", e);
        return Err(anyhow::anyhow!("Failed to start messengers"));
    }

    // Start report scheduler
    // let report_scheduler = ReportScheduler::new(db_pool.clone(), messenger_manager_arc.clone());
    // if let Err(e) = report_scheduler.start().await {
    //     tracing::error!("Failed to start report scheduler: {:?}", e);
    //     return Err(anyhow::anyhow!("Failed to start report scheduler"));
    // }

    // build our application with a route
    let app = app::build_router(AppState {
        version: "0.1.0".to_string(),
        db_pool,
        jwt_secret: config.jwt_secret,
        chat_relay_secret: config.chat_relay_secret,
        messenger_manager: Some(messenger_manager_arc),
        lang,
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

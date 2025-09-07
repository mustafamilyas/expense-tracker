use anyhow::Result;

mod app;
mod error;
mod routes;

#[tokio::main]
async fn main() -> Result<()> {
     // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = app::build_router();

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
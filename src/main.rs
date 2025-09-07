use anyhow::Result;

mod app;
mod error;

#[tokio::main]
async fn main() -> Result<()> {
     // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = app::build_router();

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
    Ok(())
}

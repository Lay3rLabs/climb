mod args;
mod config;
mod error;
mod handlers;
mod prelude;
mod state;

use args::CliArgs;
use axum::{
    routing::{get, post},
    Router,
};
use clap::Parser;
use state::AppState;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();
    let state = AppState::new(args).await.unwrap();

    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .with_max_level(state.config.tracing_level)
        .init();

    let cors = state
        .config
        .cors_allowed_origins
        .clone()
        .map(|allowed_origins| {
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::predicate(
                    move |origin, _parts| {
                        // using a predicate so we can handle any port
                        origin
                            .to_str()
                            .map(|s| {
                                allowed_origins
                                    .iter()
                                    .any(|allowed_origin| s.starts_with(allowed_origin))
                            })
                            .unwrap_or(false)
                    },
                ))
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
        });

    // build our application with a single route
    let mut router = Router::new()
        .route("/status", get(handlers::status::status))
        .route("/credit", post(handlers::credit::credit))
        .fallback(handlers::not_found::not_found)
        .with_state(state.clone());

    if let Some(cors) = cors {
        router = router.layer(cors);
    }

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", state.config.port))
        .await
        .unwrap();

    tracing::info!("Listening on: {}", listener.local_addr().unwrap());

    axum::serve(listener, router).await.unwrap();
}

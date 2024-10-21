use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{config::Config, handlers, state::AppState};

pub async fn make_router(config: Config) -> Result<Router> {
    let state = AppState::new(config).await?;

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
        .layer(TraceLayer::new_for_http())
        .route("/status", get(handlers::status::status))
        .route("/credit", post(handlers::credit::credit))
        .fallback(handlers::not_found::not_found)
        .with_state(state.clone());

    if let Some(cors) = cors {
        router = router.layer(cors);
    }

    Ok(router)
}

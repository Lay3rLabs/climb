use anyhow::Result;
use clap::Parser;
use layer_climb_faucet::{
    args::CliArgs,
    config::{Config, ConfigInit},
    router::make_router,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();
    let config = Config::try_from(ConfigInit::load(args.config).await?)?;
    let port = config.port;

    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .with_max_level(config.tracing_level)
        .init();

    let router = make_router(config).await?;

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port)).await?;

    tracing::info!("Listening on: {}", listener.local_addr()?);

    axum::serve(listener, router).await?;

    Ok(())
}

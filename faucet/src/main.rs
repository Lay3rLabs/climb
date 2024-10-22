use anyhow::Result;
use clap::Parser;
use layer_climb_faucet::{
    args::CliArgs,
    config::{Config, ConfigInit},
    router::make_router,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();
    let config = Config::try_from(ConfigInit::load(args.config).await?)?;
    let port = config.port;

    let mut tracing_env = tracing_subscriber::EnvFilter::from_default_env();
    for directive in &config.tracing_directives {
        tracing_env = tracing_env.add_directive(directive.parse().unwrap());
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_target(false),
        )
        .with(tracing_env)
        .try_init()
        .unwrap();

    let router = make_router(config).await?;

    let listener = tokio::net::TcpListener::bind(&format!("0.0.0.0:{}", port)).await?;

    tracing::info!("Listening on: {}", listener.local_addr()?);

    axum::serve(listener, router).await?;

    Ok(())
}

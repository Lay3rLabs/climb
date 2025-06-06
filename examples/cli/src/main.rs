mod args;
mod commands;
mod config;
mod context;

use anyhow::Result;
use args::{CliArgs, Command, ContractArgs, FaucetArgs, PoolArgs, WalletArgs};
use clap::Parser;
use commands::faucet::FaucetCommand;
use context::AppContext;
use layer_climb_cli::command::{ContractLog, WalletCommand, WalletLog};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Load the .env file before anything, in case it's used by args
    if dotenvy::dotenv().is_err() {
        eprintln!("Failed to load .env file");
    }

    // load the args before setting up the logger, since it uses the log level
    let args = CliArgs::parse();

    let mut tracing_env = tracing_subscriber::EnvFilter::from_default_env();
    for directive in args.tracing_directives.split(',').map(|s| s.trim()) {
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

    // now we can get our context, which will contain the args too
    let mut ctx = AppContext::new(args).await?;

    match &ctx.args.command {
        Command::Wallet(WalletArgs { command }) => {
            let command = match &command {
                WalletCommand::Show { mnemonic } => {
                    if mnemonic.is_none() {
                        WalletCommand::Show {
                            mnemonic: ctx.client_mnemonic().ok(),
                        }
                    } else {
                        command.clone()
                    }
                }
                _ => command.clone(),
            };
            command
                .run(ctx.any_client().await?, &mut ctx.rng, |line| match line {
                    WalletLog::Create { addr, mnemonic } => {
                        tracing::info!("Created wallet with address: {}", addr);
                        tracing::info!("Mnemonic: {}", mnemonic);
                    }
                    WalletLog::Show { addr, balances } => {
                        tracing::info!("Wallet address: {}", addr);
                        for balance in balances {
                            tracing::info!("{}: {}", balance.denom, balance.amount);
                        }
                    }
                    WalletLog::Balance { addr, balance } => {
                        tracing::info!("Wallet address: {}", addr);
                        tracing::info!("{}: {}", balance.denom, balance.amount);
                    }
                    WalletLog::AllBalances { addr, balances } => {
                        tracing::info!("Wallet address: {}", addr);
                        for balance in balances {
                            tracing::info!("{}: {}", balance.denom, balance.amount);
                        }
                    }
                    WalletLog::Transfer {
                        to,
                        amount,
                        denom,
                        tx_resp,
                    } => {
                        tracing::info!("Transfer successful, tx hash: {}", tx_resp.txhash);
                        tracing::info!("Sent {} {} to {}", amount, denom, to);
                    }
                })
                .await?;
        }
        Command::Contract(ContractArgs { command }) => {
            command
                .run(ctx.any_client().await?, |line| match line {
                    ContractLog::Upload { code_id, tx_resp } => {
                        tracing::info!("Uploaded contract with code id: {}", code_id);
                        tracing::info!("Tx hash: {}", tx_resp.txhash);
                    }
                    ContractLog::Instantiate { addr, tx_resp } => {
                        tracing::info!("Instantiated contract at address: {}", addr);
                        tracing::info!("Tx hash: {}", tx_resp.txhash);
                    }
                    ContractLog::Execute { tx_resp } => {
                        tracing::info!("Executed contract");
                        tracing::info!("Tx hash: {}", tx_resp.txhash);
                    }
                    ContractLog::Query { response } => {
                        tracing::info!("Contract query response: {}", response);
                    }
                })
                .await?;
        }
        Command::Faucet(FaucetArgs { command }) => {
            let command = match &command {
                FaucetCommand::Tap { to, amount, denom } => {
                    if to.is_none() {
                        FaucetCommand::Tap {
                            to: Some(ctx.client_addr().await?.to_string()),
                            amount: *amount,
                            denom: denom.clone(),
                        }
                    } else {
                        command.clone()
                    }
                }
            };
            command.run(&ctx).await?;
        }
        Command::Pool(PoolArgs { command }) => {
            command.run(&ctx).await?;
        }
    }

    Ok(())
}

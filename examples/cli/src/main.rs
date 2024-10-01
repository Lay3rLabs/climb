mod args;
mod commands;
mod config;
mod context;

use anyhow::Result;
use args::{CliArgs, Command, ContractArgs, FaucetArgs, PoolArgs, WalletArgs};
use clap::Parser;
use context::AppContext;
use layer_climb_cli::command::{ContractLog, WalletLog};

#[tokio::main]
async fn main() -> Result<()> {
    // Load the .env file before anything, in case it's used by args
    if dotenvy::dotenv().is_err() {
        eprintln!("Failed to load .env file");
    }

    // load the args before setting up the logger, since it uses the log level
    let args = CliArgs::parse();

    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .with_max_level(tracing::Level::from(args.log_level))
        .init();

    // now we can get our context, which will contain the args too
    let mut ctx = AppContext::new(args).await?;

    match &ctx.args.command {
        Command::Wallet(WalletArgs { command }) => {
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
            command.run(&ctx).await?;
        }
        Command::Pool(PoolArgs { command }) => {
            command.run(&ctx).await?;
        }
    }

    Ok(())
}

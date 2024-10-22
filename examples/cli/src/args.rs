use clap::Parser;
use clap::{Args, Subcommand};
use layer_climb_cli::command::{ContractCommand, WalletCommand};

use crate::commands::faucet::FaucetCommand;
use crate::commands::pool::PoolCommand;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[arg(long, value_enum, default_value_t = TargetEnvironment::Local)]
    pub target_env: TargetEnvironment,

    /// Set the comma-separated list of tracing directives
    #[arg(long, default_value = "info")]
    pub tracing_directives: String,

    #[command(subcommand)]
    /// The command to run
    pub command: Command,
}

#[derive(Clone, Subcommand)]
pub enum Command {
    /// Wallet subcommands
    Wallet(WalletArgs),
    /// Contract subcommands
    Contract(ContractArgs),
    /// Faucet subcommands
    Faucet(FaucetArgs),
    /// Pool subcommands
    Pool(PoolArgs),
}

#[derive(Clone, Args)]
pub struct WalletArgs {
    #[command(subcommand)]
    pub command: WalletCommand,
}

#[derive(Clone, Args)]
pub struct ContractArgs {
    #[command(subcommand)]
    pub command: ContractCommand,
}

#[derive(Clone, Args)]
pub struct FaucetArgs {
    #[command(subcommand)]
    pub command: FaucetCommand,
}

#[derive(Clone, Args)]
pub struct PoolArgs {
    #[command(subcommand)]
    pub command: PoolCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum TargetEnvironment {
    Local,
    Testnet,
}

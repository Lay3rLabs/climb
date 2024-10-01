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

    /// Set the logging level
    #[arg(long, value_enum, default_value_t = LogLevel::Info)]
    //#[arg(long, value_enum, default_value_t = LogLevel::Debug)]
    pub log_level: LogLevel,

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

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum TargetEnvironment {
    Local,
    Testnet,
}

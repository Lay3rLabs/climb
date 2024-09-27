use std::{path::PathBuf, str::FromStr};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use cosmwasm_std::Coin;
use deadpool::managed::{Manager, Pool};
use layer_climb::{pool::SigningClientPoolManager, prelude::*};
use serde::{Deserialize, Serialize};

// https://docs.rs/clap/latest/clap/_derive/_tutorial/chapter_0/index.html

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, value_enum, default_value_t = TargetEnvironment::Local)]
    pub target_env: TargetEnvironment,

    /// Set the logging level
    #[arg(long, value_enum, default_value_t = LogLevel::Info)]
    //#[arg(long, value_enum, default_value_t = LogLevel::Debug)]
    pub log_level: LogLevel,

    #[command(subcommand)]
    pub command: Command,
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

#[derive(Clone, Subcommand)]
pub enum Command {
    /// Shows the wallet balance and address
    WalletShow {},

    /// Taps the faucet to get some funds
    TapFaucet {
        #[arg(long)]
        amount: Option<u128>,
    },

    /// Taps the faucet multiple times to get some funds (tests pool)
    MultiTapFaucet {
        #[arg(long)]
        amount: Option<u128>,
    },

    /// Generates a random wallet.
    /// Shows the mnemonic and address.
    GenerateWallet,

    /// Uploads a contract to the chain
    UploadContract {
        /// Path to the .wasm file to upload
        #[arg(long)]
        wasm_file: PathBuf,
    },

    /// Instantiates a contract on the chain
    InstantiateContract {
        /// The code ID of the contract, obtained from `upload-contract`
        #[arg(long)]
        code_id: u64,
        /// The instantiation message, as a json-encoded string
        #[arg(long)]
        msg: Option<String>,
        /// Optional label for the contract
        #[arg(long)]
        label: Option<String>,
        /// Optional funds to send, if not set will use the chain gas denom
        #[arg(long)]
        funds_denom: Option<String>,
        /// Optional funds to send, if not set no funds will be sent
        #[arg(long)]
        funds_amount: Option<String>,
    },

    /// Executes a contract on the chain
    ExecuteContract {
        /// The address of the contract, obtained from `instantiate-contract`
        #[arg(long)]
        address: String,
        /// The execution message, as a json-encoded string
        #[arg(long)]
        msg: Option<String>,
        /// Optional funds to send, if not set will use the chain gas denom
        #[arg(long)]
        funds_denom: Option<String>,
        /// Optional funds to send, if not set no funds will be sent
        #[arg(long)]
        funds_amount: Option<String>,
    },

    /// Queries a contract on the chain
    QueryContract {
        /// The address of the contract, obtained from `instantiate-contract`
        #[arg(long)]
        address: String,
        /// The query message, as a json-encoded string
        #[arg(long)]
        msg: Option<String>,
    },
}

pub struct Opt {
    pub command: Command,
    pub chain_config: ChainConfig,
    mnemonic: String,
    faucet_config: FaucetConfig,
}

impl Opt {
    pub async fn new(args: Args) -> Result<Self> {
        let mnemonic = match args.target_env {
            TargetEnvironment::Local => std::env::var("LOCAL_MNEMONIC"),
            TargetEnvironment::Testnet => std::env::var("TEST_MNEMONIC"),
        }
        .context("Mnemonic not found")?;

        let configs: Config = serde_json::from_str(include_str!("../config.json"))
            .context("Failed to parse config")?;

        let chain_config = match args.target_env {
            TargetEnvironment::Local => configs.chains.local,
            TargetEnvironment::Testnet => configs.chains.testnet,
        }
        .context(format!(
            "Chain config for environment {:?} not found",
            args.target_env
        ))?;

        Ok(Opt {
            command: args.command,
            chain_config,
            mnemonic,
            faucet_config: configs.faucet,
        })
    }

    pub fn signer(&self) -> Result<KeySigner> {
        KeySigner::new_mnemonic_str(&self.mnemonic, None)
    }

    pub async fn address(&self) -> Result<Address> {
        self.chain_config
            .address_from_pub_key(&self.signer()?.public_key().await?)
    }

    pub async fn query_client(&self) -> Result<QueryClient> {
        QueryClient::new(self.chain_config.clone()).await
    }

    pub async fn signing_client(&self) -> Result<SigningClient> {
        SigningClient::new(self.chain_config.clone(), self.signer()?).await
    }

    pub async fn faucet_client(&self) -> Result<SigningClient> {
        let signer = KeySigner::new_mnemonic_str(&self.faucet_config.mnemonic, None)?;
        SigningClient::new(self.chain_config.clone(), signer).await
    }

    pub async fn faucet_pool(&self) -> Result<Pool<SigningClientPoolManager>> {
        let manager = SigningClientPoolManager::new_mnemonic(
            self.faucet_config.mnemonic.clone(),
            self.chain_config.clone(),
            None,
        );
        Pool::builder(manager)
            .max_size(100)
            .build()
            .map_err(|e| e.into())
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    pub chains: ChainConfigs,
    pub faucet: FaucetConfig,
}
#[derive(Debug, Deserialize, Serialize)]
struct ChainConfigs {
    pub local: Option<ChainConfig>,
    pub testnet: Option<ChainConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FaucetConfig {
    pub mnemonic: String,
}

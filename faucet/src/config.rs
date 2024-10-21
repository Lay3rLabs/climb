use anyhow::Context;
use layer_climb::prelude::*;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

/// This is first loaded via the filepath (faucet.toml by default, settable via CLI arg --config)
/// Then, the .env file is loaded if specified in the `dotenv` field
/// Finally, any env vars are loaded, overwriting any previous values if found
/// For the environment variables, the prefix `FAUCET_` is used and the field name in all caps
/// For example, the field `log_level` would be set by the env var `FAUCET_LOG_LEVEL`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigInit {
    pub dotenv: Option<PathBuf>,
    pub log_level: LogLevel,
    pub cors_allowed_origins: Option<Vec<String>>,
    pub port: u16,
    pub concurrency: usize,
    pub memo: Option<String>,
    pub mnemonic_env_var: String,
    /// Should be in micro-units, e.g. 25_000_000 would be a typical value
    /// this should be an integer-string
    pub credit_amount: String,
    /// if not set, will use `chain_gas_denom`
    pub credit_denom: Option<String>,
    pub chain_id: ChainId,
    pub chain_rpc_endpoint: String,
    pub chain_grpc_endpoint: String,
    /// not micro-units, e.g. 0.025 would be a typical value
    pub chain_gas_price: f32,
    pub chain_gas_denom: String,
    pub chain_address_kind: ConfigChainAddrKindName,
    /// only applicable if `chain_address_kind` is `cosmos`
    pub chain_address_bech32_prefix: Option<String>,
    /// The minimum balance of credit to maintain on each concurrent client
    /// set this to as low as reasonable, to reduce unnecessary transfers
    /// this should be an integer-string
    pub minimum_credit_balance_threshhold: String,
    /// The amount to send to top up each concurrent client
    /// set this to as high as reasonable, to reduce unnecessary transfers
    /// this should be an integer-string
    pub minimum_credit_balance_topup: String,
}

// This is simply derived from ConfigInit in a format that's more reasonable to pass around
#[derive(Debug, Clone)]
pub struct Config {
    pub tracing_level: tracing::Level,
    pub mnemonic: String,
    pub cors_allowed_origins: Option<Vec<String>>,
    pub port: u16,
    pub concurrency: usize,
    pub memo: Option<String>,
    pub chain_config: ChainConfig,
    pub credit: Coin,
    pub minimum_credit_balance_threshhold: u128,
    pub minimum_credit_balance_topup: u128,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ConfigChainAddrKindName {
    Cosmos,
    Eth,
}

impl FromStr for ConfigChainAddrKindName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "cosmos" => Ok(Self::Cosmos),
            "eth" => Ok(Self::Eth),
            _ => Err(format!("Unknown chain address kind: {}", s)),
        }
    }
}

impl ConfigInit {
    pub async fn load(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        Self::load_inner(tokio::fs::read_to_string(path.into()).await?)
    }

    pub fn load_sync(path: impl Into<PathBuf>) -> anyhow::Result<Self> {
        Self::load_inner(std::fs::read_to_string(path.into())?)
    }

    fn load_inner(s: String) -> anyhow::Result<Self> {
        // first load from the file
        let mut config: Self = toml::from_str(&s)?;

        // next load .env file, if specified
        if let Some(dotenv) = &config.dotenv {
            if dotenvy::from_filename(dotenv).is_err() {
                eprintln!("Failed to load .env file");
            }
        }

        // now update from env vars - none of these should fail, just silently ignore if not found
        if let Ok(log_level) = std::env::var("FAUCET_LOG_LEVEL") {
            config.log_level = log_level.parse().unwrap_or(config.log_level);
        }

        if let Ok(cors_allowed_origins) = std::env::var("FAUCET_CORS_ALLOWED_ORIGINS") {
            config.cors_allowed_origins = Some(
                cors_allowed_origins
                    .split(',')
                    .map(|s| s.to_string())
                    .collect(),
            );
        }

        if let Ok(port) = std::env::var("FAUCET_PORT") {
            config.port = port.parse().unwrap_or(config.port);
        }

        if let Ok(concurrency) = std::env::var("FAUCET_CONCURRENCY") {
            config.concurrency = concurrency.parse().unwrap_or(config.concurrency);
        }

        if let Ok(memo) = std::env::var("FAUCET_MEMO") {
            config.memo = Some(memo);
        }

        if let Ok(chain_id) = std::env::var("FAUCET_CHAIN_ID") {
            config.chain_id = ChainId::new(chain_id);
        }

        if let Ok(chain_rpc_endpoint) = std::env::var("FAUCET_CHAIN_RPC_ENDPOINT") {
            config.chain_rpc_endpoint = chain_rpc_endpoint;
        }

        if let Ok(chain_grpc_endpoint) = std::env::var("FAUCET_CHAIN_GRPC_ENDPOINT") {
            config.chain_grpc_endpoint = chain_grpc_endpoint;
        }

        if let Ok(gas_price) = std::env::var("FAUCET_CHAIN_GAS_PRICE") {
            config.chain_gas_price = gas_price.parse().context("Failed to parse gas price")?;
        }

        if let Ok(gas_denom) = std::env::var("FAUCET_CHAIN_GAS_DENOM") {
            config.chain_gas_denom = gas_denom;
        }

        if let Ok(chain_address_kind) = std::env::var("FAUCET_CHAIN_ADDRESS_KIND") {
            config.chain_address_kind = chain_address_kind
                .parse()
                .map_err(|_| anyhow::anyhow!("Failed to parse chain address kind"))?;
        }

        if let Ok(chain_address_bech32_prefix) = std::env::var("FAUCET_CHAIN_ADDRESS_BECH32_PREFIX")
        {
            config.chain_address_bech32_prefix = Some(chain_address_bech32_prefix);
        }

        if let Ok(credit_amount) = std::env::var("FAUCET_CREDIT_AMOUNT") {
            config.credit_amount = credit_amount;
        }

        if let Ok(credit_denom) = std::env::var("FAUCET_CREDIT_DENOM") {
            config.credit_denom = Some(credit_denom);
        }

        if let Ok(mnemonic_env_var) = std::env::var("FAUCET_MNEMONIC_ENV_VAR") {
            config.mnemonic_env_var = mnemonic_env_var;
        }

        Ok(config)
    }
}

impl TryFrom<ConfigInit> for Config {
    type Error = anyhow::Error;

    fn try_from(config: ConfigInit) -> anyhow::Result<Self> {
        let credit_denom = config
            .credit_denom
            .unwrap_or(config.chain_gas_denom.clone());
        let credit = new_coin(config.credit_amount, credit_denom);

        Ok(Self {
            tracing_level: tracing::Level::from(config.log_level),
            cors_allowed_origins: config.cors_allowed_origins,
            port: config.port,
            concurrency: config.concurrency,
            memo: config.memo,
            credit,
            mnemonic: std::env::var(&config.mnemonic_env_var)
                .context(format!("Missing mnemonic in {}", config.mnemonic_env_var))?,
            chain_config: ChainConfig {
                chain_id: config.chain_id,
                rpc_endpoint: config.chain_rpc_endpoint,
                grpc_endpoint: config.chain_grpc_endpoint,
                grpc_web_endpoint: None,
                gas_price: config.chain_gas_price,
                gas_denom: config.chain_gas_denom,
                address_kind: match config.chain_address_kind {
                    ConfigChainAddrKindName::Cosmos => AddrKind::Cosmos {
                        prefix: config
                            .chain_address_bech32_prefix
                            .context("Missing bech32 prefix")?,
                    },
                    ConfigChainAddrKindName::Eth => AddrKind::Eth,
                },
            },
            minimum_credit_balance_threshhold: config
                .minimum_credit_balance_threshhold
                .parse()
                .context("couldn't parse minimum credit balance threshhold")?,
            minimum_credit_balance_topup: config
                .minimum_credit_balance_topup
                .parse()
                .context("couldn't parse minimum credit topup")?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warn" => Ok(Self::Warn),
            "error" => Ok(Self::Error),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
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

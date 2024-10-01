use anyhow::{Context, Result};
use layer_climb::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub chains: ChainConfigs,
    pub faucet: FaucetConfig,
}

impl Config {
    pub fn load() -> Result<Self> {
        serde_json::from_str(include_str!("../config.json")).context("Failed to parse config")
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChainConfigs {
    pub local: Option<ChainConfig>,
    pub testnet: Option<ChainConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FaucetConfig {
    pub mnemonic: String,
}
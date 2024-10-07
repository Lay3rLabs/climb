use anyhow::{Context, Result};
use layer_climb::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub local: Option<ChainInfo>,
    pub testnet: Option<ChainInfo>,
}

impl Config {
    pub fn load() -> Result<Self> {
        serde_json::from_str(include_str!("../../config.json")).context("Failed to parse config")
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChainInfo {
    pub chain: ChainConfig,
    pub faucet: FaucetConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FaucetConfig {
    pub mnemonic: String,
}

#![allow(clippy::too_many_arguments)]
#![allow(warnings)]
mod bindings;

use anyhow::{Context, Result};
use layer_climb::prelude::*;
use serde::{Deserialize, Serialize};

// https://docs.rs/wit-bindgen/0.37.0/wit_bindgen/macro.generate.html

wit_bindgen::generate!({
    world: "example-world",
    path: "./wit",
    generate_all,
    //async: true,
});

struct Component;

impl Guest for Component {
    fn run() -> std::result::Result<String, String> {
        let config = Config::load().map_err(|e| e.to_string())?;

        wstd::runtime::block_on(async move {
            let client = get_client(config).await.map_err(|e| e.to_string())?;
            let height = client.block_height().await.map_err(|e| e.to_string())?;
            Ok(format!("Block height: {}", height))
        })
    }
}

async fn get_client(config: Config) -> Result<QueryClient> {
    let chain_config = config
        .local
        .context("local chain not configured")?
        .chain
        .clone();

    QueryClient::new(chain_config.clone(), None).await
}

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

bindings::export!(Component);

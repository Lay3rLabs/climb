use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: ChainId,
    pub rpc_endpoint: Option<String>,
    pub grpc_endpoint: Option<String>,
    // if not specified, will fallback to `grpc_endpoint`
    pub grpc_web_endpoint: Option<String>,
    // not micro-units, e.g. 0.025 would be a typical value
    pub gas_price: f32,
    pub gas_denom: String,
    pub address_kind: AddrKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AddrKind {
    Cosmos { prefix: String },
    Evm,
}

impl std::hash::Hash for AddrKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            AddrKind::Cosmos { prefix } => {
                1u32.hash(state);
                prefix.hash(state);
            }
            AddrKind::Evm => {
                2u32.hash(state);
            }
        }
    }
}

impl ChainConfig {
    pub fn ibc_client_revision(&self) -> Result<u64> {
        // > Tendermint chains wishing to use revisions to maintain persistent IBC connections even across height-resetting upgrades
        // > must format their chainIDs in the following manner: {chainID}-{revision_number}
        // - https://github.com/cosmos/ibc-go/blob/main/docs/docs/01-ibc/01-overview.md#ibc-client-heights
        Ok(self
            .chain_id
            .as_str()
            .split("-")
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_default())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct ChainId(String);
impl ChainId {
    pub fn new(id: impl ToString) -> Self {
        Self(id.to_string())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ChainId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s))
    }
}

impl Display for ChainId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

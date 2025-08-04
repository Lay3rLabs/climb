use layer_climb_address::AddrKind;
use layer_climb_config::{util::set_port_in_url, ChainConfig, ChainId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebChainConfig {
    pub chain_id: ChainId,
    pub rpc_endpoint: Option<String>,
    pub grpc_endpoint: Option<String>,
    // if not specified, will fallback to `grpc_endpoint`
    pub grpc_web_endpoint: Option<String>,
    // needed for wallets like Keplr
    pub rest_endpoint: Option<String>,
    // not micro-units, e.g. 0.025 would be a typical value
    pub gas_price: f32,
    pub gas_denom: String,
    pub address_kind: AddrKind,
}

impl From<WebChainConfig> for ChainConfig {
    fn from(web_chain_config: WebChainConfig) -> Self {
        Self {
            chain_id: web_chain_config.chain_id,
            rpc_endpoint: web_chain_config.rpc_endpoint,
            grpc_endpoint: web_chain_config.grpc_endpoint,
            grpc_web_endpoint: web_chain_config.grpc_web_endpoint,
            gas_price: web_chain_config.gas_price,
            gas_denom: web_chain_config.gas_denom,
            address_kind: web_chain_config.address_kind,
        }
    }
}

/// This implementation uses the rpc endpoint as the rest endpoint
/// but changes the port to 1317
impl From<ChainConfig> for WebChainConfig {
    fn from(chain_config: ChainConfig) -> Self {
        let rest_endpoint = chain_config
            .rpc_endpoint
            .as_ref()
            .map(|endpoint| set_port_in_url(endpoint, 1317).unwrap());

        Self {
            chain_id: chain_config.chain_id,
            rpc_endpoint: chain_config.rpc_endpoint,
            rest_endpoint,
            grpc_endpoint: chain_config.grpc_endpoint,
            grpc_web_endpoint: chain_config.grpc_web_endpoint,
            gas_price: chain_config.gas_price,
            gas_denom: chain_config.gas_denom,
            address_kind: chain_config.address_kind,
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum KeplrError {
    #[error("keplr: missing chain")]
    MissingChain,
    #[error("keplr: failed enable")]
    FailedEnable,
    #[error("keplr: doesn't exist")]
    NoExist,
    #[error("keplr: no signer")]
    NoSigner,
    #[error("keplr: unknown {0}")]
    Unknown(String),
    #[error("keplr: technical {0}")]
    Technical(String),
}

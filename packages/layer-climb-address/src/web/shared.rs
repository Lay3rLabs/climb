use layer_climb_config::{util::set_port_in_url, AddrKind, ChainConfig, ChainId};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebChainConfig {
    pub chain_id: ChainId,
    pub rpc_endpoint: String,
    pub grpc_endpoint: String,
    pub rest_endpoint: String,
    pub gas_amount: String,
    pub gas_denom: String,
    pub address_kind: AddrKind,
}

impl From<WebChainConfig> for ChainConfig {
    fn from(web_chain_config: WebChainConfig) -> Self {
        Self {
            chain_id: web_chain_config.chain_id,
            rpc_endpoint: web_chain_config.rpc_endpoint,
            grpc_endpoint: web_chain_config.grpc_endpoint,
            gas_amount: web_chain_config.gas_amount,
            gas_denom: web_chain_config.gas_denom,
            address_kind: web_chain_config.address_kind,
        }
    }
}

/// This implementation uses the rpc endpoint as the rest endpoint
/// but changes the port to 1317
impl From<ChainConfig> for WebChainConfig {
    fn from(chain_config: ChainConfig) -> Self {
        let rest_endpoint = set_port_in_url(&chain_config.rpc_endpoint, 1317).unwrap();

        Self {
            chain_id: chain_config.chain_id,
            rpc_endpoint: chain_config.rpc_endpoint,
            rest_endpoint,
            grpc_endpoint: chain_config.grpc_endpoint,
            gas_amount: chain_config.gas_amount,
            gas_denom: chain_config.gas_denom,
            address_kind: chain_config.address_kind,
        }
    }
}

use crate::{address::Address, key::PublicKey};
use anyhow::Result;
use layer_climb_config::{AddrKind, ChainConfig};

pub trait ConfigAddressExt {
    fn parse_address(&self, value: &str) -> Result<Address>;
    fn address_from_pub_key(&self, pub_key: &PublicKey) -> Result<Address>;
}

impl ConfigAddressExt for ChainConfig {
    fn parse_address(&self, value: &str) -> Result<Address> {
        match &self.address_kind {
            AddrKind::Cosmos { prefix } => Address::new_cosmos_string(value, Some(prefix)),
            AddrKind::Eth => Address::new_eth_string(value),
        }
    }

    fn address_from_pub_key(&self, pub_key: &PublicKey) -> Result<Address> {
        match &self.address_kind {
            AddrKind::Cosmos { prefix } => Address::new_cosmos_pub_key(pub_key, prefix),
            AddrKind::Eth => Address::new_eth_pub_key(pub_key),
        }
    }
}

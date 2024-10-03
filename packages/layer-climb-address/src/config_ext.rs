use crate::{address::Address, key::PublicKey};
use anyhow::Result;
use layer_climb_config::ChainConfig;

pub trait ConfigAddressExt {
    fn parse_address(&self, value: &str) -> Result<Address>;
    fn address_from_pub_key(&self, pub_key: &PublicKey) -> Result<Address>;
}

impl ConfigAddressExt for ChainConfig {
    fn parse_address(&self, value: &str) -> Result<Address> {
        Address::try_from_value(value, &self.address_kind)
    }

    fn address_from_pub_key(&self, pub_key: &PublicKey) -> Result<Address> {
        Address::try_from_pub_key(pub_key, &self.address_kind)
    }
}

use crate::{address::Address, key::PublicKey};
use anyhow::Result;
use layer_climb_config::{AddrKind, ChainConfig};

pub trait ConfigAddressExt {
    fn parse_address(&self, value: &str) -> Result<Address>;
    fn address_from_pub_key(&self, pub_key: &PublicKey) -> Result<Address>;
}

impl ConfigAddressExt for AddrKind {
    fn parse_address(&self, value: &str) -> Result<Address> {
        Address::try_from_str(value, self)
    }

    fn address_from_pub_key(&self, pub_key: &PublicKey) -> Result<Address> {
        Address::try_from_pub_key(pub_key, self)
    }
}

impl ConfigAddressExt for ChainConfig {
    fn parse_address(&self, value: &str) -> Result<Address> {
        self.address_kind.parse_address(value)
    }

    fn address_from_pub_key(&self, pub_key: &PublicKey) -> Result<Address> {
        self.address_kind.address_from_pub_key(pub_key)
    }
}

use crate::PublicKey;
use anyhow::{anyhow, bail, Context, Result};
use layer_climb_config::AddrKind;
use serde::{Deserialize, Serialize};
use std::{hash::Hash, str::FromStr};
use subtle_encoding::bech32;

/// The canonical type used everywhere for addresses
/// Display is implemented as plain string
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Address {
    Cosmos {
        bech32_addr: String,
        // prefix is the first part of the bech32 address
        prefix_len: usize,
    },
    Eth(AddrEth),
}

impl Hash for Address {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Address::Cosmos { .. } => {
                1u32.hash(state);
            }
            Address::Eth(_) => {
                2u32.hash(state);
            }
        }
        self.to_string().hash(state);
    }
}

impl Address {
    // used internally for validation across both ways of creating addresses:
    // 1. parsing from a string
    // 2. creating from a public key
    fn new_cosmos(bytes: Vec<u8>, prefix: &str) -> Result<Self> {
        if !prefix.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9')) {
            bail!("expected prefix to be lowercase alphanumeric characters only");
        }

        if bytes.len() > 255 {
            bail!(
                "account ID should be at most 255 bytes long, but was {} bytes long",
                bytes.len()
            );
        }

        let bech32_addr = bech32::encode(prefix, bytes);

        Ok(Self::Cosmos {
            bech32_addr,
            prefix_len: prefix.len(),
        })
    }
    // if the prefix is supplied, this will attempt to validate the address against the prefix to ensure they match
    // if you just have a public key, use new_cosmos_pub_key instead
    pub fn new_cosmos_string(value: &str, prefix: Option<&str>) -> Result<Self> {
        let (decoded_prefix, decoded_bytes) = if value.starts_with(|c: char| c.is_uppercase()) {
            bech32::decode_upper(value)
        } else {
            bech32::decode(value)
        }
        .context(format!("invalid bech32: '{}'", value))?;

        if matches!(prefix, Some(prefix) if prefix != decoded_prefix) {
            bail!("Address prefix does not match expected prefix");
        }

        Self::new_cosmos(decoded_bytes, &decoded_prefix)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Address::Cosmos { bech32_addr, .. } => {
                let (_, bytes) = bech32::decode(bech32_addr).unwrap();
                bytes
            }
            Address::Eth(addr_eth) => addr_eth.as_bytes().to_vec(),
        }
    }

    /// if you just have a string address, use new_cosmos_string instead
    pub fn new_cosmos_pub_key(pub_key: &PublicKey, prefix: &str) -> Result<Self> {
        match pub_key {
            PublicKey::Secp256k1(encoded_point) => {
                let id = tendermint::account::Id::from(*encoded_point);
                Self::new_cosmos(id.as_bytes().to_vec(), prefix)
            }
            _ => Err(anyhow!(
                "Invalid public key type, currently only supports secp256k1"
            )),
        }
    }

    pub fn cosmos_prefix(&self) -> Result<&str> {
        match self {
            Address::Cosmos {
                prefix_len,
                bech32_addr,
            } => Ok(&bech32_addr[..*prefix_len]),
            Address::Eth(_) => Err(anyhow!("Address is not cosmos")),
        }
    }

    pub fn new_eth_string(value: &str) -> Result<Self> {
        let addr_eth: AddrEth = value.parse()?;
        Ok(Self::Eth(addr_eth))
    }

    /// if you just have a string address, use parse_eth instead
    pub fn new_eth_pub_key(_pub_key: &PublicKey) -> Result<Self> {
        bail!("TODO - implement eth address from public key");
    }

    pub fn into_cosmos(&self, new_prefix: &str) -> Result<Self> {
        match self {
            Address::Cosmos { bech32_addr, .. } => {
                if self.cosmos_prefix()? == new_prefix {
                    Ok(self.clone())
                } else {
                    Self::new_cosmos_string(bech32_addr, Some(new_prefix))
                }
            }
            Address::Eth(_) => {
                bail!("TODO - implement eth to cosmos addr");
            }
        }
    }

    pub fn into_eth(&self) -> Result<Self> {
        match self {
            Address::Eth(_) => Ok(self.clone()),
            Address::Cosmos { .. } => {
                bail!("TODO - implement cosmos to eth addr");
            }
        }
    }

    pub fn try_from_str(value: &str, addr_kind: &AddrKind) -> Result<Self> {
        match addr_kind {
            AddrKind::Cosmos { prefix } => Self::new_cosmos_string(value, Some(prefix)),
            AddrKind::Eth => Self::new_eth_string(value),
        }
    }

    pub fn try_from_pub_key(pub_key: &PublicKey, addr_kind: &AddrKind) -> Result<Address> {
        match addr_kind {
            AddrKind::Cosmos { prefix } => Address::new_cosmos_pub_key(pub_key, prefix),
            AddrKind::Eth => Address::new_eth_pub_key(pub_key),
        }
    }
}

// the display impl ignores the kind
impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cosmos { bech32_addr, .. } => {
                write!(f, "{}", bech32_addr)
            }
            Self::Eth(addr_eth) => {
                write!(f, "{}", addr_eth)
            }
        }
    }
}

///// Ethereum address
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AddrEth([u8; 20]);

impl AddrEth {
    pub fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    pub fn new_vec(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 20 {
            bail!("Invalid length for eth address");
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn as_bytes(&self) -> [u8; 20] {
        self.0
    }
}

impl std::fmt::Display for AddrEth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl FromStr for AddrEth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.len() != 42 {
            bail!("Invalid length for eth address");
        }
        if !s.starts_with("0x") {
            bail!("Invalid prefix for eth address");
        }
        let bytes = hex::decode(&s[2..])?;
        Self::new_vec(bytes)
    }
}

impl TryFrom<Address> for AddrEth {
    type Error = anyhow::Error;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Eth(addr_eth) => Ok(addr_eth),
            Address::Cosmos { .. } => bail!("Address must be Eth - use into_eth() instead"),
        }
    }
}

impl From<AddrEth> for Address {
    fn from(addr: AddrEth) -> Self {
        Self::Eth(addr)
    }
}

#[cfg(test)]
mod test {
    use super::{AddrEth, Address};

    // TODO get addresses that are actually the same underlying public key

    const TEST_COSMOS_STR: &str = "osmo1h5qke5tzc0fgz93wcxg8da2en3advfect0gh4a";
    const TEST_COSMOS_PREFIX: &str = "osmo";
    const TEST_ETH_STR: &str = "0xb794f5ea0ba39494ce839613fffba74279579268";

    #[test]
    fn test_basic_roundtrip_eth() {
        let test_string = TEST_ETH_STR;
        let addr_eth: AddrEth = test_string.parse().unwrap();
        let addr: Address = addr_eth.into();

        assert_eq!(addr.to_string(), test_string);

        let addr_eth_2: AddrEth = addr.try_into().unwrap();
        assert_eq!(addr_eth_2, addr_eth);
    }

    #[test]
    fn test_basic_roundtrip_cosmos() {
        let test_string = TEST_COSMOS_STR;
        let test_prefix = TEST_COSMOS_PREFIX;
        let addr = Address::new_cosmos_string(test_string, None).unwrap();

        assert_eq!(addr.to_string(), test_string);
        assert_eq!(addr.cosmos_prefix().unwrap(), test_prefix);
    }

    #[test]
    fn test_convert_eth_to_cosmos() {
        // let test_string = "0xb794f5ea0ba39494ce839613fffba74279579268";
        // let addr_bytes:AddrEth = test_string.try_into().unwrap();
        // let addr_string:AddrString = (&addr_bytes).into();
        // let addr_string_cosmos = addr_string.convert_into_cosmos("osmo".to_string()).unwrap();
        // assert_eq!(addr_string_cosmos.to_string(), "osmo1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrsll0sqv");
    }

    #[test]
    fn test_convert_cosmos_to_eth() {
        // let test_string = "osmo1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrsll0sqv";
        // let account_id:AccountId = test_string.parse().unwrap();
        // let addr_string:AddrString = (&account_id).try_into().unwrap();
        // let addr_string_eth = addr_string.convert_into_eth().unwrap();
        // assert_eq!(addr_string_eth.to_string(), "0xb794f5ea0ba39494ce839613fffba74279579268");
    }
}

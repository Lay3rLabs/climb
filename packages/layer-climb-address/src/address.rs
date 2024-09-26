use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::{hash::Hash, str::FromStr};

/// The canonical type used everywhere for addresses
/// Display is implemented as plain string
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Address {
    Cosmos(cosmrs::AccountId),
    Eth(AddrEth),
}

impl Hash for Address {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Address::Cosmos(_) => {
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
    // this will attempt to validate the address.
    // If you want to avoid that, use the From trait
    pub fn new_cosmos(value: &str, prefix: &str) -> Result<Self> {
        let account_id: cosmrs::AccountId = value.parse().map_err(|e| anyhow!("{e:?}"))?;
        if account_id.prefix() != prefix {
            bail!("Address prefix does not match expected prefix");
        }

        Ok(Self::Cosmos(account_id))
    }

    pub fn new_eth(value: &str) -> Result<Self> {
        let addr_eth: AddrEth = value.parse()?;
        Ok(Self::Eth(addr_eth))
    }

    pub fn new_cosmos_pub_key(pub_key: &cosmrs::crypto::PublicKey, prefix: &str) -> Result<Self> {
        let account_id = pub_key.account_id(prefix).map_err(|e| anyhow!("{e:?}"))?;
        Ok(Self::Cosmos(account_id))
    }

    pub fn new_eth_pub_key(_pub_key: &cosmrs::crypto::PublicKey) -> Result<Self> {
        bail!("TODO - implement eth address from public key");
    }

    pub fn into_cosmos(&self, prefix: &str) -> Result<Self> {
        match self {
            Address::Cosmos(account_id) => {
                if account_id.prefix() == prefix {
                    Ok(self.clone())
                } else {
                    let account_id = cosmrs::AccountId::new(prefix, &account_id.to_bytes())
                        .map_err(|e| anyhow!("{e:?}"))?;
                    Ok(Self::Cosmos(account_id))
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
            Address::Cosmos(_) => {
                bail!("TODO - implement cosmos to eth addr");
            }
        }
    }
}

// the display impl ignores the kind
impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cosmos(account_id) => {
                write!(f, "{}", account_id)
            }
            Self::Eth(addr_eth) => {
                write!(f, "{}", addr_eth)
            }
        }
    }
}

///// Cosmos address
impl TryFrom<Address> for cosmrs::AccountId {
    type Error = anyhow::Error;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Cosmos(account_id) => Ok(account_id),
            Address::Eth(_) => bail!("Address must be Cosmos - use into_cosmos() instead"),
        }
    }
}

impl From<cosmrs::AccountId> for Address {
    fn from(account_id: cosmrs::AccountId) -> Self {
        Self::Cosmos(account_id)
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
            Address::Cosmos(_) => bail!("Address must be Eth - use into_eth() instead"),
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
    use cosmrs::AccountId;

    // TODO get addresses that are actually the same underlying public key

    const TEST_COSMOS_STR: &str = "osmo1h5qke5tzc0fgz93wcxg8da2en3advfect0gh4a";
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
        let account_id: AccountId = test_string.parse().unwrap();
        let addr: Address = account_id.clone().into();

        assert_eq!(addr.to_string(), test_string);

        let account_id_2: AccountId = addr.try_into().unwrap();
        assert_eq!(account_id_2, account_id);
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

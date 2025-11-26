mod cosmos;
mod evm;

use crate::error::{ClimbAddressError, Result};
use cosmwasm_schema::cw_serde;
use std::hash::Hash;

pub use cosmos::CosmosAddr;
pub use evm::EvmAddr;

/// The canonical type used everywhere for addresses
/// Display is implemented as plain string
// cw_serde implements Serialize/Deserialize, Clone, Debug
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[derive(Eq, PartialOrd, Ord, Hash, bincode::Encode, bincode::Decode)]
#[cw_serde]
pub enum Address {
    Cosmos(CosmosAddr),
    Evm(EvmAddr),
}

impl Address {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Address::Cosmos(addr_cosmos) => addr_cosmos.to_vec(),
            Address::Evm(addr_evm) => addr_evm.as_bytes().to_vec(),
        }
    }

    pub fn try_from_str(value: &str, addr_kind: &AddrKind) -> Result<Self> {
        match addr_kind {
            AddrKind::Cosmos { prefix } => {
                CosmosAddr::new_str(value, Some(prefix)).map(Self::Cosmos)
            }
            AddrKind::Evm => EvmAddr::new_str(value).map(Self::Evm),
        }
    }

    pub fn try_from_pub_key(
        pub_key: &tendermint::PublicKey,
        addr_kind: &AddrKind,
    ) -> Result<Address> {
        match addr_kind {
            AddrKind::Cosmos { prefix } => {
                CosmosAddr::new_pub_key(pub_key, prefix).map(Self::Cosmos)
            }
            AddrKind::Evm => EvmAddr::new_pub_key(pub_key).map(Self::Evm),
        }
    }
}

// the display impl ignores the kind
impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cosmos(addr_cosmos) => {
                write!(f, "{addr_cosmos}")
            }
            Self::Evm(addr_evm) => {
                write!(f, "{addr_evm}")
            }
        }
    }
}

// TryFrom<Address>
impl TryFrom<Address> for EvmAddr {
    type Error = ClimbAddressError;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Evm(addr) => Ok(addr),
            Address::Cosmos(_) => Err(ClimbAddressError::NotEvm),
        }
    }
}

impl TryFrom<Address> for CosmosAddr {
    type Error = ClimbAddressError;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Cosmos(addr) => Ok(addr),
            Address::Evm(_) => Err(ClimbAddressError::NotCosmos),
        }
    }
}

impl TryFrom<Address> for alloy_primitives::Address {
    type Error = ClimbAddressError;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Evm(addr) => Ok(addr.into()),
            Address::Cosmos(_) => Err(ClimbAddressError::NotEvm),
        }
    }
}

impl TryFrom<Address> for cosmwasm_std::Addr {
    type Error = ClimbAddressError;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Cosmos(addr) => Ok(addr.into()),
            Address::Evm(_) => Err(ClimbAddressError::NotCosmos),
        }
    }
}

// Into<Address>
impl From<EvmAddr> for Address {
    fn from(addr: EvmAddr) -> Self {
        Self::Evm(addr)
    }
}

impl From<CosmosAddr> for Address {
    fn from(addr: CosmosAddr) -> Self {
        Self::Cosmos(addr)
    }
}

impl From<alloy_primitives::Address> for Address {
    fn from(addr: alloy_primitives::Address) -> Self {
        Self::Evm(addr.into())
    }
}

impl TryFrom<cosmwasm_std::Addr> for Address {
    type Error = ClimbAddressError;

    fn try_from(addr: cosmwasm_std::Addr) -> Result<Self> {
        Ok(Self::Cosmos(addr.try_into()?))
    }
}

impl TryFrom<&cosmwasm_std::Addr> for Address {
    type Error = ClimbAddressError;

    fn try_from(addr: &cosmwasm_std::Addr) -> Result<Self> {
        Ok(Self::Cosmos(addr.try_into()?))
    }
}

#[cw_serde]
#[derive(Eq, Hash)]
pub enum AddrKind {
    Cosmos { prefix: String },
    Evm,
}

impl AddrKind {
    pub fn parse_address(&self, value: &str) -> Result<Address> {
        Address::try_from_str(value, self)
    }

    pub fn address_from_pub_key(&self, pub_key: &tendermint::PublicKey) -> Result<Address> {
        Address::try_from_pub_key(pub_key, self)
    }
}

#[cfg(test)]
mod test {
    use super::{Address, CosmosAddr, EvmAddr};

    // TODO get addresses that are actually the same underlying public key

    const TEST_COSMOS_STR: &str = "osmo1h5qke5tzc0fgz93wcxg8da2en3advfect0gh4a";
    const TEST_COSMOS_PREFIX: &str = "osmo";
    const TEST_EVM_STR: &str = "0xb794f5ea0ba39494ce839613fffba74279579268";

    #[test]
    fn test_basic_roundtrip_evm() {
        let test_string = TEST_EVM_STR;
        let addr_evm: EvmAddr = test_string.parse().unwrap();
        let addr: Address = addr_evm.clone().into();

        assert_eq!(addr.to_string(), test_string);

        let addr_evm_2: EvmAddr = addr.clone().try_into().unwrap();
        assert_eq!(addr_evm_2, addr_evm);

        // serde should be as hex string
        assert_eq!(
            serde_json::to_string(&addr_evm).unwrap(),
            format!("\"{test_string}\"")
        );
        assert_eq!(
            serde_json::from_str::<EvmAddr>(&format!("\"{test_string}\"")).unwrap(),
            addr_evm
        );
    }

    #[test]
    fn test_basic_roundtrip_cosmos() {
        let cosmos_addr = CosmosAddr::new_str(TEST_COSMOS_STR, None).unwrap();
        let addr: Address = cosmos_addr.clone().into();

        assert_eq!(addr.to_string(), TEST_COSMOS_STR);
        assert_eq!(cosmos_addr.prefix(), TEST_COSMOS_PREFIX);
    }

    #[test]
    fn test_serde_roundtrip_cosmos() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct TestStruct {
            addr: CosmosAddr,
        }

        let test_struct: TestStruct =
            serde_json::from_str(&format!(r#"{{ "addr": "{TEST_COSMOS_STR}"}}"#)).unwrap();
        let addr: Address = test_struct.addr.clone().into();

        let test_struct_2 = TestStruct {
            addr: addr.try_into().unwrap(),
        };

        assert_eq!(
            serde_json::to_string(&test_struct_2).unwrap().trim(),
            format!(r#"{{"addr":"{TEST_COSMOS_STR}"}}"#).trim()
        );
        assert_eq!(test_struct_2.addr.prefix(), TEST_COSMOS_PREFIX);
    }

    #[test]
    fn test_convert_evm_to_cosmos() {
        // let test_string = "0xb794f5ea0ba39494ce839613fffba74279579268";
        // let addr_bytes:EvmAddr= test_string.try_into().unwrap();
        // let addr_string:AddrString = (&addr_bytes).into();
        // let addr_string_cosmos = addr_string.convert_into_cosmos("osmo".to_string()).unwrap();
        // assert_eq!(addr_string_cosmos.to_string(), "osmo1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrsll0sqv");
    }

    #[test]
    fn test_convert_cosmos_to_evm() {
        // let test_string = "osmo1suhgf5svhu4usrurvxzlgn54ksxmn8gljarjtxqnapv8kjnp4nrsll0sqv";
        // let account_id:AccountId = test_string.parse().unwrap();
        // let addr_string:AddrString = (&account_id).try_into().unwrap();
        // let addr_string_evm = addr_string.convert_into_evm().unwrap();
        // assert_eq!(addr_string_evm.to_string(), "0xb794f5ea0ba39494ce839613fffba74279579268");
    }
}

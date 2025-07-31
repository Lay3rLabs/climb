use crate::PublicKey;
use anyhow::{anyhow, bail, Context, Result};
use cosmwasm_schema::{cw_schema, cw_serde};
use layer_climb_config::AddrKind;
use std::{borrow::Cow, hash::Hash, str::FromStr};
use subtle_encoding::bech32;
use utoipa::ToSchema;

/// The canonical type used everywhere for addresses
/// Display is implemented as plain string
#[derive(ToSchema, Eq)]
#[cw_serde]
pub enum Address {
    Cosmos {
        bech32_addr: String,
        // prefix is the first part of the bech32 address
        prefix_len: usize,
    },
    Evm(AddrEvm),
}

impl Hash for Address {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Address::Cosmos { .. } => {
                1u32.hash(state);
            }
            Address::Evm(_) => {
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
        .context(format!("invalid bech32: '{value}'"))?;

        if matches!(prefix, Some(prefix) if prefix != decoded_prefix) {
            bail!(
                "Address prefix \"{}\" does not match expected prefix \"{}\"",
                decoded_prefix,
                prefix.unwrap()
            );
        }

        Self::new_cosmos(decoded_bytes, &decoded_prefix)
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Address::Cosmos { bech32_addr, .. } => {
                let (_, bytes) = bech32::decode(bech32_addr).unwrap();
                bytes
            }
            Address::Evm(addr_evm) => addr_evm.as_bytes().to_vec(),
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
            Address::Evm(_) => Err(anyhow!("Address is not cosmos")),
        }
    }

    pub fn new_evm_string(value: &str) -> Result<Self> {
        let addr_evm: AddrEvm = value.parse()?;
        Ok(Self::Evm(addr_evm))
    }

    /// if you just have a string address, use parse_evm instead
    pub fn new_evm_pub_key(_pub_key: &PublicKey) -> Result<Self> {
        bail!("TODO - implement evm address from public key");
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
            Address::Evm(_) => {
                bail!("TODO - implement evm to cosmos addr");
            }
        }
    }

    pub fn into_evm(&self) -> Result<Self> {
        match self {
            Address::Evm(_) => Ok(self.clone()),
            Address::Cosmos { .. } => {
                bail!("TODO - implement cosmos to evm addr");
            }
        }
    }

    pub fn try_from_str(value: &str, addr_kind: &AddrKind) -> Result<Self> {
        match addr_kind {
            AddrKind::Cosmos { prefix } => Self::new_cosmos_string(value, Some(prefix)),
            AddrKind::Evm => Self::new_evm_string(value),
        }
    }

    pub fn try_from_pub_key(pub_key: &PublicKey, addr_kind: &AddrKind) -> Result<Address> {
        match addr_kind {
            AddrKind::Cosmos { prefix } => Address::new_cosmos_pub_key(pub_key, prefix),
            AddrKind::Evm => Address::new_evm_pub_key(pub_key),
        }
    }
}

// the display impl ignores the kind
impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cosmos { bech32_addr, .. } => {
                write!(f, "{bech32_addr}")
            }
            Self::Evm(addr_evm) => {
                write!(f, "{addr_evm}")
            }
        }
    }
}

impl From<alloy_primitives::Address> for Address {
    fn from(addr: alloy_primitives::Address) -> Self {
        Self::Evm(addr.into())
    }
}

impl TryFrom<Address> for alloy_primitives::Address {
    type Error = anyhow::Error;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Evm(addr_evm) => Ok(addr_evm.into()),
            Address::Cosmos { .. } => Err(anyhow!("Expected EVM address, got Cosmos")),
        }
    }
}

/// EVM address
// we implement our own Serialize/Deserialize to ensure it is serialized as a hex string
// so we need to manually implement the cw_serde derives from https://github.com/CosmWasm/cosmwasm/blob/fa5439a9e4e6884abe1e76f04443a95961eaa73f/packages/schema-derive/src/cw_serde.rs#L47C5-L61C7
#[derive(ToSchema, Clone, Debug, PartialEq, Eq)]
pub struct AddrEvm([u8; 20]);

impl cw_schema::Schemaifier for AddrEvm {
    #[inline]
    fn visit_schema(visitor: &mut cw_schema::SchemaVisitor) -> cw_schema::DefinitionReference {
        let node = cw_schema::Node {
            name: Cow::Borrowed(std::any::type_name::<Self>()),
            description: None,
            value: cw_schema::NodeType::String,
        };

        visitor.insert(Self::id(), node)
    }
}

impl cosmwasm_schema::schemars::JsonSchema for AddrEvm {
    fn schema_name() -> String {
        "AddrEvm".into()
    }

    fn json_schema(
        _generator: &mut cosmwasm_schema::schemars::r#gen::SchemaGenerator,
    ) -> cosmwasm_schema::schemars::schema::Schema {
        cosmwasm_schema::schemars::schema::Schema::Object(
            cosmwasm_schema::schemars::schema::SchemaObject {
                instance_type: Some(cosmwasm_schema::schemars::schema::SingleOrVec::Single(
                    Box::new(cosmwasm_schema::schemars::schema::InstanceType::String),
                )),
                format: Some("hex".into()),
                ..Default::default()
            },
        )
    }
}

impl AddrEvm {
    pub fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    pub fn new_vec(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 20 {
            bail!("Invalid length for EVM address");
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn as_bytes(&self) -> [u8; 20] {
        self.0
    }
}

impl std::fmt::Display for AddrEvm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl FromStr for AddrEvm {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        if s.len() != 42 {
            bail!("Invalid length for EVM address");
        }
        if !s.starts_with("0x") {
            bail!("Invalid prefix for EVM address");
        }
        let bytes = hex::decode(&s[2..])?;
        Self::new_vec(bytes)
    }
}

impl TryFrom<Address> for AddrEvm {
    type Error = anyhow::Error;

    fn try_from(addr: Address) -> Result<Self> {
        match addr {
            Address::Evm(addr_evm) => Ok(addr_evm),
            Address::Cosmos { .. } => bail!("Address must be EVM - use into_evm() instead"),
        }
    }
}

impl From<AddrEvm> for Address {
    fn from(addr: AddrEvm) -> Self {
        Self::Evm(addr)
    }
}

impl From<alloy_primitives::Address> for AddrEvm {
    fn from(addr: alloy_primitives::Address) -> Self {
        Self(**addr)
    }
}

impl From<AddrEvm> for alloy_primitives::Address {
    fn from(addr: AddrEvm) -> Self {
        alloy_primitives::Address::new(addr.0)
    }
}

impl serde::Serialize for AddrEvm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for AddrEvm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use super::{AddrEvm, Address};

    // TODO get addresses that are actually the same underlying public key

    const TEST_COSMOS_STR: &str = "osmo1h5qke5tzc0fgz93wcxg8da2en3advfect0gh4a";
    const TEST_COSMOS_PREFIX: &str = "osmo";
    const TEST_EVM_STR: &str = "0xb794f5ea0ba39494ce839613fffba74279579268";

    #[test]
    fn test_basic_roundtrip_evm() {
        let test_string = TEST_EVM_STR;
        let addr_evm: AddrEvm = test_string.parse().unwrap();
        let addr: Address = addr_evm.clone().into();

        assert_eq!(addr.to_string(), test_string);

        let addr_evm_2: AddrEvm = addr.clone().try_into().unwrap();
        assert_eq!(addr_evm_2, addr_evm);

        // serde should be as hex string
        assert_eq!(
            serde_json::to_string(&addr_evm).unwrap(),
            format!("\"{test_string}\"")
        );
        assert_eq!(
            serde_json::from_str::<AddrEvm>(&format!("\"{test_string}\"")).unwrap(),
            addr_evm
        );
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
    fn test_convert_evm_to_cosmos() {
        // let test_string = "0xb794f5ea0ba39494ce839613fffba74279579268";
        // let addr_bytes:AddrEvm = test_string.try_into().unwrap();
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

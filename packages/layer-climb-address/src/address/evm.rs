use std::{borrow::Cow, str::FromStr};

use crate::error::{AddressError, Result};
use cosmwasm_schema::cw_schema;

/// EVM address
// we implement our own Serialize/Deserialize to ensure it is serialized as a hex string
// so we need to manually implement the cw_serde derives from https://github.com/CosmWasm/cosmwasm/blob/fa5439a9e4e6884abe1e76f04443a95961eaa73f/packages/schema-derive/src/cw_serde.rs#L47C5-L61C7
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, bincode::Encode, bincode::Decode)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "cw-storage", derive(cw_storage_plus::NewTypeKey))]
pub struct EvmAddr([u8; 20]);

impl EvmAddr {
    pub fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    pub fn new_vec(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() != 20 {
            return Err(AddressError::InvalidFormat(format!(
                "Invalid length for EVM address: expected 20 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    pub fn new_pub_key(_pub_key: &tendermint::PublicKey) -> Result<Self> {
        Err(AddressError::UnsupportedPubKey)
    }

    pub fn new_str(s: &str) -> Result<Self> {
        let decoded = const_hex::decode(s.trim())
            .map_err(|e| AddressError::InvalidFormat(format!("invalid hex: {e}")))?;
        Self::new_vec(decoded)
    }

    pub fn as_bytes(&self) -> [u8; 20] {
        self.0
    }
}
impl cw_schema::Schemaifier for EvmAddr {
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

impl cosmwasm_schema::schemars::JsonSchema for EvmAddr {
    fn schema_name() -> String {
        "EvmAddr".into()
    }

    fn json_schema(
        _generator: &mut cosmwasm_schema::schemars::r#gen::SchemaGenerator,
    ) -> cosmwasm_schema::schemars::schema::Schema {
        cosmwasm_schema::schemars::schema::Schema::Object(
            cosmwasm_schema::schemars::schema::SchemaObject {
                instance_type: Some(cosmwasm_schema::schemars::schema::SingleOrVec::Single(
                    Box::new(cosmwasm_schema::schemars::schema::InstanceType::String),
                )),
                format: Some("evm-address".into()),
                ..Default::default()
            },
        )
    }
}

impl std::fmt::Display for EvmAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", const_hex::encode(self.0))
    }
}

impl FromStr for EvmAddr {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new_str(s)
    }
}

impl serde::Serialize for EvmAddr {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for EvmAddr {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

// From/into impls
impl From<alloy_primitives::Address> for EvmAddr {
    fn from(addr: alloy_primitives::Address) -> Self {
        Self(**addr)
    }
}

impl From<EvmAddr> for alloy_primitives::Address {
    fn from(addr: EvmAddr) -> Self {
        alloy_primitives::Address::new(addr.0)
    }
}

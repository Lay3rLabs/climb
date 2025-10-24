use std::{borrow::Cow, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};
use cosmwasm_schema::cw_schema;
use subtle_encoding::bech32;

/// Cosmos address
// we implement our own Serialize/Deserialize to ensure it is serialized as a hex string
// so we need to manually implement the cw_serde derives from https://github.com/CosmWasm/cosmwasm/blob/fa5439a9e4e6884abe1e76f04443a95961eaa73f/packages/schema-derive/src/cw_serde.rs#L47C5-L61C7
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct CosmosAddr {
    bech32_addr: String,
    // prefix is the first part of the bech32 address
    prefix_len: usize,
}

// used internally for validation across both ways of creating addresses:
// 1. parsing from a string
// 2. creating from a public key
impl CosmosAddr {
    pub fn new_unchecked(value: impl ToString, prefix_len: usize) -> Self {
        Self {
            bech32_addr: value.to_string(),
            prefix_len,
        }
    }

    pub fn new_bytes(bytes: Vec<u8>, prefix: &str) -> Result<Self> {
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

        Ok(Self {
            bech32_addr,
            prefix_len: prefix.len(),
        })
    }

    /// if you just have a string address, use new_cosmos_string instead
    pub fn new_pub_key(pub_key: &tendermint::PublicKey, prefix: &str) -> Result<Self> {
        match pub_key {
            tendermint::PublicKey::Secp256k1(encoded_point) => {
                let id = tendermint::account::Id::from(*encoded_point);
                Self::new_bytes(id.as_bytes().to_vec(), prefix)
            }
            _ => Err(anyhow!(
                "Invalid public key type, currently only supports secp256k1"
            )),
        }
    }

    // if the prefix is supplied, this will attempt to validate the address against the prefix to ensure they match
    // if you just have a public key, use new_cosmos_pub_key instead
    pub fn new_str(value: &str, prefix: Option<&str>) -> Result<Self> {
        let (decoded_prefix, decoded_bytes) = if value.starts_with(|c: char| c.is_uppercase()) {
            bech32::decode_upper(value)
        } else {
            bech32::decode(value)
        }
        .context(format!("invalid bech32: '{value}'"))?;

        if let Some(prefix) = prefix {
            if decoded_prefix != prefix {
                bail!(
                    "Address prefix \"{}\" does not match expected prefix \"{}\"",
                    decoded_prefix,
                    prefix
                );
            }
        }

        Self::new_bytes(decoded_bytes, &decoded_prefix)
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let (_, bytes) = bech32::decode(&self.bech32_addr).unwrap();
        bytes
    }

    pub fn prefix(&self) -> &str {
        &self.bech32_addr[..self.prefix_len]
    }

    pub fn change_prefix(&self, new_prefix: &str) -> Result<Self> {
        if self.prefix() == new_prefix {
            Ok(self.clone())
        } else {
            Self::new_str(&self.bech32_addr, Some(new_prefix))
        }
    }
}

// the display impl ignores the prefix
impl std::fmt::Display for CosmosAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.bech32_addr)
    }
}

impl FromStr for CosmosAddr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::new_str(s, None)
    }
}

impl serde::Serialize for CosmosAddr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for CosmosAddr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl cw_schema::Schemaifier for CosmosAddr {
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

impl cosmwasm_schema::schemars::JsonSchema for CosmosAddr {
    fn schema_name() -> String {
        "CosmosAddr".into()
    }

    fn json_schema(
        _generator: &mut cosmwasm_schema::schemars::r#gen::SchemaGenerator,
    ) -> cosmwasm_schema::schemars::schema::Schema {
        cosmwasm_schema::schemars::schema::Schema::Object(
            cosmwasm_schema::schemars::schema::SchemaObject {
                instance_type: Some(cosmwasm_schema::schemars::schema::SingleOrVec::Single(
                    Box::new(cosmwasm_schema::schemars::schema::InstanceType::String),
                )),
                format: Some("cosmos-address".into()),
                ..Default::default()
            },
        )
    }
}

// From/Into impls
impl TryFrom<cosmwasm_std::Addr> for CosmosAddr {
    type Error = anyhow::Error;

    fn try_from(addr: cosmwasm_std::Addr) -> Result<Self> {
        Self::new_str(addr.as_str(), None)
    }
}

impl From<CosmosAddr> for cosmwasm_std::Addr {
    fn from(addr: CosmosAddr) -> Self {
        cosmwasm_std::Addr::unchecked(addr.to_string())
    }
}

impl TryFrom<&cosmwasm_std::Addr> for CosmosAddr {
    type Error = anyhow::Error;

    fn try_from(addr: &cosmwasm_std::Addr) -> Result<Self> {
        Self::new_str(addr.as_str(), None)
    }
}

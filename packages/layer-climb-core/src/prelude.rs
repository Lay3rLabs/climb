// local "prelude" that isn't exported
// some of these may be exported in the main prelude
pub(crate) use crate::network::apply_grpc_height;
pub(crate) use anyhow::{anyhow, bail, Context, Result};
use cosmwasm_std::Uint256;
pub(crate) use layer_climb_address::Address;
pub(crate) use layer_climb_config::*;
pub(crate) use layer_climb_proto::{proto_into_any, proto_into_bytes, Message};

// common types
pub use crate::{
    cache::ClimbCache,
    contract_helpers::contract_str_to_msg,
    error::{
        AccountError, ClimbError, ConfigError, NetworkError, TransactionError, ValidationError,
    },
    events::CosmosTxEvents,
    querier::{Connection, ConnectionMode, QueryClient, QueryRequest},
    signing::SigningClient,
    transaction::TxBuilder,
};

#[cfg(not(target_arch = "wasm32"))]
pub use crate::pool::*;

// Common types that can be confusing between different proto files.
// standardized here. In cases where we want helper methods, use extension traits
// so that we don't have to deal with confusion between types.

/// helper function to create a Coin
pub fn new_coin(amount: impl ToString, denom: impl ToString) -> layer_climb_proto::Coin {
    layer_climb_proto::Coin {
        denom: denom.to_string(),
        amount: amount.to_string(),
    }
}

/// helper function to create a vec of coins from an iterator of tuples
/// where the first is the amount, and the second is the denom.
/// Example:
/// ```ignore
/// use layer_climb::prelude::*;
///
/// new_coins([
///     ("uusd", "100"),
///     ("uslay", "200")
/// ])
/// ```
pub fn new_coins(
    coins: impl IntoIterator<Item = (impl ToString, impl ToString)>,
) -> Vec<layer_climb_proto::Coin> {
    coins
        .into_iter()
        .map(|(amount, denom)| new_coin(amount, denom))
        .collect()
}

pub trait ProtoCoinExt {
    fn try_to_cosmwasm_coin(&self) -> Result<cosmwasm_std::Coin>;
}

impl ProtoCoinExt for layer_climb_proto::Coin {
    fn try_to_cosmwasm_coin(&self) -> Result<cosmwasm_std::Coin> {
        Ok(cosmwasm_std::Coin {
            denom: self.denom.clone(),
            amount: self
                .amount
                .parse::<Uint256>()
                .map_err(|e| anyhow!("{e:?}"))?,
        })
    }
}

pub trait CosmwasmCoinExt {
    fn to_proto_coin(&self) -> layer_climb_proto::Coin;
}

impl CosmwasmCoinExt for cosmwasm_std::Coin {
    fn to_proto_coin(&self) -> layer_climb_proto::Coin {
        layer_climb_proto::Coin {
            denom: self.denom.clone(),
            amount: self.amount.to_string(),
        }
    }
}

/// A useful abstraction when we have either a Signing or Query client
/// but need to delay the decision of requiring it to be a SigningClient until runtime.
pub enum AnyClient {
    Signing(SigningClient),
    Query(QueryClient),
}

impl AnyClient {
    // helper method to get the signing client via ref when we know it's what we have
    // will panic if called with a QueryClient (for error handling in that case, use TryInto)
    pub fn as_signing(&self) -> &SigningClient {
        match self {
            Self::Signing(client) => client,
            Self::Query(_) => panic!("Expected SigningClient, got QueryClient"),
        }
    }

    // helper method to get the query client via ref
    pub fn as_querier(&self) -> &QueryClient {
        match self {
            Self::Query(client) => client,
            Self::Signing(client) => &client.querier,
        }
    }
}

impl From<SigningClient> for AnyClient {
    fn from(client: SigningClient) -> Self {
        Self::Signing(client)
    }
}

impl From<QueryClient> for AnyClient {
    fn from(client: QueryClient) -> Self {
        Self::Query(client)
    }
}

impl TryFrom<AnyClient> for SigningClient {
    type Error = anyhow::Error;

    fn try_from(value: AnyClient) -> Result<Self> {
        match value {
            AnyClient::Signing(client) => Ok(client),
            AnyClient::Query(_) => Err(anyhow!("Expected SigningClient, got QueryClient")),
        }
    }
}

impl From<AnyClient> for QueryClient {
    fn from(client: AnyClient) -> Self {
        match client {
            AnyClient::Query(client) => client,
            AnyClient::Signing(client) => client.querier,
        }
    }
}

// local "prelude" that isn't exported
// some of these may be exported in the main prelude
pub(crate) use crate::network::apply_grpc_height;
pub(crate) use anyhow::{anyhow, bail, Context, Result};
pub(crate) use layer_climb_address::{Address, ConfigAddressExt};
pub(crate) use layer_climb_config::*;
pub(crate) use layer_climb_proto::{proto_into_any, proto_into_bytes, Message};

// common types
pub use crate::{
    contract_helpers::contract_str_to_msg,
    events::CosmosTxEvents,
    querier::{QueryClient, QueryRequest},
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

/// A useful abstraction when we have either a Signing or Query client
/// but need to delay the decision of requiring it to be a SigningClient until runtime.
pub enum AnyClient {
    Signing(SigningClient),
    Query(QueryClient),
}

impl AnyClient {
    pub fn as_signing(&self) -> &SigningClient {
        match self {
            Self::Signing(client) => client,
            Self::Query(_) => panic!("Expected SigningClient, got QueryClient"),
        }
    }

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

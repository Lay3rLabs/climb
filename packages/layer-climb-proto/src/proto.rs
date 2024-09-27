// Exported in the root because they're commonly used
pub use cosmos_sdk_proto::{
    cosmos::base::v1beta1::Coin,
    tendermint::google::protobuf::{Any, Duration, Timestamp},
    traits::{Message, MessageExt, Name},
};

// do these really not exist in cosmos_sdk_proto?
// TODO - double-check to see if these are only used for ibc stuff, if so, move over there
pub use ibc_proto::ibc::core::client::v1::Height as RevisionHeight;
pub use ibc_proto::ibc::core::commitment::v1::{MerklePrefix, MerkleProof, MerkleRoot};

// the rest are all in distinct modules
pub mod block {
    // uhh... yeah... this is a bit of a mess
    pub use cosmos_sdk_proto::{
        cosmos::base::tendermint::v1beta1::Block as SdkBlock,
        cosmos::base::tendermint::v1beta1::Header as SdkHeader,
        tendermint::types::Block as TendermintBlock, tendermint::types::Header as TendermintHeader,
    };
}

pub mod auth {
    pub use cosmos_sdk_proto::cosmos::auth::v1beta1::*;
}

pub mod bank {
    pub use cosmos_sdk_proto::cosmos::bank::v1beta1::*;
}

pub mod tendermint {
    pub use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::*;
    pub use cosmos_sdk_proto::tendermint::{
        abci::{Event, EventAttribute},
        types::{BlockId, SignedHeader, Validator, ValidatorSet},
    };
    pub use tendermint_proto::crypto;
}

pub mod abci {
    pub use cosmos_sdk_proto::cosmos::base::abci::v1beta1::*;
}

pub mod query {
    pub use cosmos_sdk_proto::cosmos::base::query::v1beta1::*;
}

pub mod crypto {
    pub use cosmos_sdk_proto::cosmos::crypto::*;
}

pub mod staking {
    pub use cosmos_sdk_proto::cosmos::staking::v1beta1::*;
}

pub mod tx {
    pub use cosmos_sdk_proto::cosmos::tx::signing::v1beta1::*;
    pub use cosmos_sdk_proto::cosmos::tx::v1beta1::*;
}

pub mod wasm {
    pub use cosmos_sdk_proto::cosmwasm::wasm::v1::*;
}

pub mod ibc {
    pub use ibc_proto::ibc::core::channel::v1 as channel;
    pub use ibc_proto::ibc::core::client::v1 as client;
    pub use ibc_proto::ibc::core::connection::v1 as connection;
    pub use ibc_proto::ibc::lightclients::tendermint::v1 as light_client;
    pub use ibc_proto::ics23;
}

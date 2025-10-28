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

pub mod tendermint {
    pub use cosmos_sdk_proto::cosmos::base::tendermint::v1beta1::*;
    pub use cosmos_sdk_proto::tendermint::{
        abci::{Event, EventAttribute},
        types::{BlockId, SignedHeader, Validator, ValidatorSet},
    };
    pub use tendermint_proto::crypto;
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

///////////////

/// Authentication of accounts and transactions.
pub mod auth {
    pub use cosmos_sdk_proto::cosmos::auth::v1beta1::*;
}

/// Granting of arbitrary privileges from one account to another.
pub mod authz {
    pub use cosmos_sdk_proto::cosmos::authz::v1beta1::*;
}

/// Balances.
pub mod bank {
    pub use cosmos_sdk_proto::cosmos::bank::v1beta1::*;
}

/// Application BlockChain Interface (ABCI).
///
/// Interface that defines the boundary between the replication engine
/// (the blockchain), and the state machine (the application).
pub mod abci {
    pub use cosmos_sdk_proto::cosmos::base::abci::v1beta1::*;
}

/// Node requests.
pub mod node {
    pub use cosmos_sdk_proto::cosmos::base::node::v1beta1::*;
}

/// Query support.
pub mod query {
    pub use cosmos_sdk_proto::cosmos::base::query::v1beta1::*;
}

/// Reflection support.
pub mod reflection {
    pub mod v1 {
        pub use cosmos_sdk_proto::cosmos::base::reflection::v1beta1::*;
    }

    pub mod v2 {
        pub use cosmos_sdk_proto::cosmos::base::reflection::v2alpha1::*;
    }
}

/// Crisis handling
pub mod crisis {
    pub use cosmos_sdk_proto::cosmos::crisis::v1beta1::*;
}

/// Cryptographic primitives.
pub mod crypto {
    /// Multi-signature support.
    pub mod multisig {
        pub use cosmos_sdk_proto::cosmos::crypto::multisig::v1beta1::*;
    }
    pub mod ed25519 {
        pub use cosmos_sdk_proto::cosmos::crypto::ed25519::*;
    }
    pub mod secp256k1 {
        pub use cosmos_sdk_proto::cosmos::crypto::secp256k1::*;
    }
    pub mod secp256r1 {
        pub use cosmos_sdk_proto::cosmos::crypto::secp256r1::*;
    }
}

/// Messages and services handling token distribution
pub mod distribution {
    pub use cosmos_sdk_proto::cosmos::distribution::v1beta1::*;
}

/// Messages and services handling evidence
pub mod evidence {
    pub use cosmos_sdk_proto::cosmos::evidence::v1beta1::*;
}

/// Allows accounts to grant fee allowances and to use fees from their accounts.
pub mod feegrant {
    pub use cosmos_sdk_proto::cosmos::feegrant::v1beta1::*;
}

/// Messages and services handling gentx's
pub mod genutil {
    pub use cosmos_sdk_proto::cosmos::genutil::v1beta1::*;
}

/// Messages and services handling governance
pub mod gov {
    pub mod v1 {
        pub use cosmos_sdk_proto::cosmos::gov::v1::*;
    }
    pub mod v1beta1 {
        pub use cosmos_sdk_proto::cosmos::gov::v1beta1::*;
    }
}

/// Messages and services handling minting
pub mod mint {
    pub use cosmos_sdk_proto::cosmos::mint::v1beta1::*;
}

/// Messages and services handling chain parameters
pub mod params {
    pub use cosmos_sdk_proto::cosmos::params::v1beta1::*;
}

/// Handling slashing parameters and unjailing
pub mod slashing {
    pub use cosmos_sdk_proto::cosmos::slashing::v1beta1::*;
}

/// Proof-of-Stake layer for public blockchains.
pub mod staking {
    pub use cosmos_sdk_proto::cosmos::staking::v1beta1::*;
}

/// Transactions.
pub mod tx {
    pub use cosmos_sdk_proto::cosmos::tx::v1beta1::*;
    /// Transaction signing support.
    pub mod signing {
        pub use cosmos_sdk_proto::cosmos::tx::signing::v1beta1::*;
    }
}

/// Services for the upgrade module.
pub mod upgrade {
    pub use cosmos_sdk_proto::cosmos::upgrade::v1beta1::*;
}

/// Services and tx's for the vesting module.
pub mod vesting {
    pub use cosmos_sdk_proto::cosmos::vesting::v1beta1::*;
}

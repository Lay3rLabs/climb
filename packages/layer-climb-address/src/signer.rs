use super::key::PublicKey;
use anyhow::{bail, Result};
use async_trait::async_trait;
use layer_climb_proto::MessageExt;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        // we assume that any signer we use in wasm32 is purely single-threaded
        #[async_trait(?Send)]
        pub trait TxSigner: Send + Sync {
            async fn sign(&self, doc: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>>;
            async fn public_key(&self) -> Result<PublicKey>;
            async fn public_key_as_proto(&self) -> Result<layer_climb_proto::Any> {
                public_key_to_proto(&self.public_key().await?)
            }
            async fn signer_info(&self, sequence: u64) -> Result<layer_climb_proto::tx::SignerInfo> {
                Ok(signer_info(self.public_key_as_proto().await?, sequence))
            }
        }
    } else {
        #[async_trait]
        pub trait TxSigner: Send + Sync {
            async fn sign(&self, doc: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>>;
            async fn public_key(&self) -> Result<PublicKey>;
            async fn public_key_as_proto(&self) -> Result<layer_climb_proto::Any> {
                public_key_to_proto(&self.public_key().await?)
            }
            async fn signer_info(&self, sequence: u64) -> Result<layer_climb_proto::tx::SignerInfo> {
                Ok(signer_info(self.public_key_as_proto().await?, sequence))
            }
        }
    }
}

fn public_key_to_proto(public_key: &PublicKey) -> Result<layer_climb_proto::Any> {
    let value = match public_key {
        tendermint::PublicKey::Ed25519(_) => layer_climb_proto::crypto::ed25519::PubKey {
            key: public_key.to_bytes(),
        }
        .to_bytes()?,
        tendermint::PublicKey::Secp256k1(_) => layer_climb_proto::crypto::secp256k1::PubKey {
            key: public_key.to_bytes(),
        }
        .to_bytes()?,
        _ => {
            bail!("Invalid public key type!")
        }
    };

    let type_url = match public_key {
        tendermint::PublicKey::Ed25519(_) => "/cosmos.crypto.ed25519.PubKey",
        tendermint::PublicKey::Secp256k1(_) => "/cosmos.crypto.secp256k1.PubKey",
        _ => {
            bail!("Invalid public key type!")
        }
    };

    Ok(layer_climb_proto::Any {
        type_url: type_url.to_string(),
        value,
    })
}

fn signer_info(
    public_key: layer_climb_proto::Any,
    sequence: u64,
) -> layer_climb_proto::tx::SignerInfo {
    layer_climb_proto::tx::SignerInfo {
        public_key: Some(public_key),
        mode_info: Some(layer_climb_proto::tx::ModeInfo {
            sum: Some(layer_climb_proto::tx::mode_info::Sum::Single(
                layer_climb_proto::tx::mode_info::Single {
                    mode: layer_climb_proto::tx::SignMode::Direct.into(),
                },
            )),
        }),
        sequence,
    }
}

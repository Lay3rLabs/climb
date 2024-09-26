use super::key::PublicKey;
use anyhow::Result;
use async_trait::async_trait;

cfg_if::cfg_if! {
    if #[cfg(feature = "web")] {
        #[async_trait(?Send)]
        pub trait TxSigner: Send + Sync {
            async fn sign(&self, doc: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>>;
            async fn public_key(&self) -> Result<PublicKey>;
        }
    } else {

        #[async_trait]
        pub trait TxSigner: Send + Sync {
            async fn sign(&self, doc: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>>;
            async fn public_key(&self) -> Result<PublicKey>;
        }
    }
}

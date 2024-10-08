// just to make IDE's happy
use anyhow::{bail, Result};
use async_trait::async_trait;
use layer_climb_config::ChainId;

use crate::{key::PublicKey, signer::TxSigner};

use super::WebChainConfig;

#[derive(Clone)]
pub struct KeplrSigner {
    pub inner: KeplrSignerInner,
}

#[derive(Clone)]
pub struct KeplrSignerInner {}

impl KeplrSigner {
    pub async fn new(_: &ChainId, _: impl Fn() + 'static) -> Result<Self> {
        Ok(Self {
            inner: KeplrSignerInner {},
        })
    }

    pub async fn add_chain(_: &WebChainConfig) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl TxSigner for KeplrSignerInner {
    async fn sign(&self, _: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>> {
        bail!("Keplr is only available in browsers");
    }

    async fn public_key(&self) -> Result<PublicKey> {
        bail!("Keplr is only available in browsers");
    }
}

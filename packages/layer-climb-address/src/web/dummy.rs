// just to make IDE's happy
use anyhow::{bail, Result};
use async_trait::async_trait;
use layer_climb_config::{ChainConfig, ChainId};

use crate::{key::PublicKey, signer::TxSigner};
pub struct KeplrSigner { }

impl KeplrSigner {
    pub async fn new(_: &ChainId) -> Result<Self> {
        Ok(Self{})
    }

    pub async fn add_chain(_: &ChainConfig) -> Result<()> {
        Ok(())
    }
}

#[async_trait]
impl TxSigner for KeplrSigner {
    async fn sign(&self, _: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>> {
        bail!("Keplr is only available in browsers");
    }

    async fn public_key(&self) -> Result<PublicKey> {
        bail!("Keplr is only available in browsers");
    }
}
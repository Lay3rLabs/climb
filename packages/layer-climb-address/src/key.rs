use super::signer::TxSigner;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bip32::DerivationPath;
use bip39::Mnemonic;
use signature::Signer;
use std::{str::FromStr, sync::LazyLock};

pub type PublicKey = tendermint::PublicKey;

// https://github.com/confio/cosmos-hd-key-derivation-spec?tab=readme-ov-file#the-cosmos-hub-path
pub static COSMOS_HUB_PATH: LazyLock<DerivationPath> =
    LazyLock::new(|| DerivationPath::from_str("m/44'/118'/0'/0/0").unwrap());

pub fn cosmos_hub_derivation(index: u32) -> Result<DerivationPath> {
    DerivationPath::from_str(&format!("m/44'/118'/0'/0/{}", index))
        .map_err(|err| anyhow!("{}", err))
}

pub struct KeySigner {
    pub key: bip32::XPrv,
}

impl KeySigner {
    pub fn new_mnemonic_iter<I, S>(mnemonic: I, derivation: Option<&DerivationPath>) -> Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut joined_str = String::new();
        for word in mnemonic {
            joined_str.push_str(word.as_ref());
            joined_str.push(' ');
        }

        Self::new_mnemonic_str(&joined_str, derivation)
    }

    pub fn new_mnemonic_str(mnemonic: &str, derivation: Option<&DerivationPath>) -> Result<Self> {
        let derivation = derivation.unwrap_or(&COSMOS_HUB_PATH);
        let mnemonic: Mnemonic = mnemonic.parse()?;
        let seed = mnemonic.to_seed("");
        let key =
            bip32::XPrv::derive_from_path(seed, derivation).map_err(|err| anyhow!("{}", err))?;

        Ok(Self { key })
    }
}

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[async_trait(?Send)]
        impl TxSigner for KeySigner {
            async fn sign(&self, msg: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>> {
                sign(self, msg).await
            }

            async fn public_key(&self) -> Result<PublicKey> {
                public_key(self).await
            }
        }

    } else {
        #[async_trait]
        impl TxSigner for KeySigner {
            async fn sign(&self, msg: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>> {
                sign(self, msg).await
            }

            async fn public_key(&self) -> Result<PublicKey> {
                public_key(self).await
            }
        }
    }
}

async fn sign(signer: &KeySigner, msg: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>> {
    let signed: k256::ecdsa::Signature = signer
        .key
        .private_key()
        .try_sign(&layer_climb_proto::proto_into_bytes(msg)?)
        .map_err(|err| anyhow!("{}", err))?;
    Ok(signed.to_vec())
}

async fn public_key(signer: &KeySigner) -> Result<PublicKey> {
    let public_key = signer.key.public_key();
    let public_key_bytes = public_key.to_bytes();
    PublicKey::from_raw_secp256k1(&public_key_bytes).context("Invalid secp256k1 public key")
}

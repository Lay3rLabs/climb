use super::signer::TxSigner;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use bip39::Mnemonic;
pub use cosmrs::crypto::PublicKey;
use cosmrs::{bip32::DerivationPath, crypto::secp256k1::SigningKey};
use std::{str::FromStr, sync::LazyLock};

// https://github.com/confio/cosmos-hd-key-derivation-spec?tab=readme-ov-file#the-cosmos-hub-path
static COSMOS_HUB_PATH: LazyLock<DerivationPath> =
    LazyLock::new(|| DerivationPath::from_str("m/44'/118'/0'/0/0").unwrap());

pub struct KeySigner {
    pub key: SigningKey,
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
        let mnemonic: Mnemonic = mnemonic.parse()?;
        let key = SigningKey::derive_from_path(
            mnemonic.to_seed(""),
            derivation.unwrap_or(&COSMOS_HUB_PATH),
        )
        .map_err(|err| anyhow!("{}", err))?;

        Ok(Self { key })
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "web")] {
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
    let signed = signer
        .key
        .sign(&layer_climb_proto::proto_into_bytes(msg)?)
        .map_err(|err| anyhow!("{}", err))?;
    Ok(signed.to_vec())
}

async fn public_key(signer: &KeySigner) -> Result<PublicKey> {
    Ok(signer.key.public_key())
}

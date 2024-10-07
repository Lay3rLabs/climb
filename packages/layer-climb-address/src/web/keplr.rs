use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use base64::prelude::*;
use layer_climb_config::ChainId;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::js_sys::Uint8Array;

use crate::{key::PublicKey, signer::TxSigner};

pub struct KeplrSigner {
    pub id: String,
}

impl KeplrSigner {
    pub async fn new(chain_id: &ChainId) -> Result<Self> {
        let id = ffi_keplr_register_signer(chain_id.as_str())
            .await
            .map_err(|e| anyhow!("{:?}", e))?;
        let id = id
            .as_string()
            .ok_or_else(|| anyhow!("Keplr signer id is not a string"))?;
        Ok(Self { id })
    }

    pub async fn add_chain(chain_config: &super::shared::WebChainConfig) -> Result<()> {
        let serialized =
            serde_wasm_bindgen::to_value(chain_config).map_err(|e| anyhow!("{:?}", e))?;

        ffi_keplr_add_chain(&serialized)
            .await
            .map_err(|e| anyhow!("{:?}", e))?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl TxSigner for KeplrSigner {
    async fn sign(&self, sign_doc: &layer_climb_proto::tx::SignDoc) -> Result<Vec<u8>> {
        #[derive(serde::Serialize)]
        struct JsSignDoc {
            #[serde(rename = "bodyBytes")]
            pub body_bytes: Vec<u8>,
            #[serde(rename = "authInfoBytes")]
            pub auth_info_bytes: Vec<u8>,
            #[serde(rename = "chainId")]
            pub chain_id: String,
            #[serde(rename = "accountNumber")]
            pub account_number: u64,
        }

        let sign_doc = JsSignDoc {
            body_bytes: sign_doc.body_bytes.clone(),
            auth_info_bytes: sign_doc.auth_info_bytes.clone(),
            chain_id: sign_doc.chain_id.clone(),
            account_number: sign_doc.account_number,
        };

        let sign_doc = serde_wasm_bindgen::to_value(&sign_doc).map_err(|e| anyhow!("{:?}", e))?;

        let signature = ffi_keplr_sign(&self.id, &sign_doc)
            .await
            .map_err(|e| anyhow!("{:?}", e))?;

        let signature = signature
            .as_string()
            .ok_or_else(|| anyhow!("Signature is not a string"))?;

        let signature_bytes = BASE64_STANDARD.decode(signature)?;

        Ok(signature_bytes)
    }

    async fn public_key(&self) -> Result<PublicKey> {
        let keplr_key = ffi_keplr_public_key(&self.id)
            .await
            .map_err(|e| anyhow!("{:?}", e))?;

        let pub_key: Vec<u8> = keplr_key.pub_key().to_vec();

        match keplr_key.algo().as_str() {
            "secp256k1" => {
                let pub_key = tendermint::public_key::PublicKey::from_raw_secp256k1(&pub_key)
                    .context("Invalid secp256k1 public key")?;
                Ok(pub_key.into())
            }
            _ => bail!("Unsupported public key algorithm: {}", keplr_key.algo()),
        }
    }
}

#[wasm_bindgen]
extern "C" {
    pub type KeplrKey;

    #[wasm_bindgen(method, getter)]
    pub fn name(this: &KeplrKey) -> String;

    #[wasm_bindgen(method, getter)]
    pub fn algo(this: &KeplrKey) -> String;

    #[wasm_bindgen(method, getter, js_name = "bech32Address")]
    pub fn bech32_addr(this: &KeplrKey) -> String;

    #[wasm_bindgen(method, getter, js_name = "ethereumHexAddress")]
    pub fn eth_addr(this: &KeplrKey) -> String;

    #[wasm_bindgen(method, getter, js_name = "isKeystone")]
    pub fn is_keystone(this: &KeplrKey) -> bool;

    #[wasm_bindgen(method, getter, js_name = "isNanoLedger")]
    pub fn is_nano_ledger(this: &KeplrKey) -> bool;

    #[wasm_bindgen(method, getter, js_name = "pubKey")]
    pub fn pub_key(this: &KeplrKey) -> Uint8Array;

    #[wasm_bindgen(method, getter)]
    pub fn address(this: &KeplrKey) -> Uint8Array;
}

#[wasm_bindgen(module = "/src/web/bindings.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn ffi_keplr_register_signer(chain_id: &str) -> std::result::Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn ffi_keplr_add_chain(chain_config: &JsValue) -> std::result::Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn ffi_keplr_sign(
        keplr_id: &str,
        sign_doc: &JsValue,
    ) -> std::result::Result<JsValue, JsValue>;

    #[wasm_bindgen(catch)]
    async fn ffi_keplr_public_key(keplr_id: &str) -> std::result::Result<KeplrKey, JsValue>;
}

use crate::prelude::*;
use anyhow::Context;
use axum::Json;
use std::{collections::HashMap, sync::atomic::Ordering};

// This gathers all the core status info we need
// but it is not the final response format, which needs to be compatible with
// the JSON format produced by CosmJS faucets
struct Status {
    chain_config: ChainConfig,
    holder: (Address, Vec<Coin>),
    distributors: HashMap<Address, Vec<Coin>>,
}

impl Status {
    pub async fn new(state: AppState) -> Result<Self> {
        let max_derivation_index = state
            .client_pool
            .manager()
            .derivation_index
            .load(Ordering::SeqCst);

        let mut distributors = HashMap::new();

        let mut holder = None;

        for derivation_index in 0..max_derivation_index {
            let addr = {
                let lock = state.distributor_addrs.lock().unwrap();
                lock.get(&derivation_index).cloned()
            };

            let addr = match addr {
                None => {
                    let derivation = cosmos_hub_derivation(derivation_index)?;

                    let signer =
                        KeySigner::new_mnemonic_str(&state.config.mnemonic, Some(&derivation))?;

                    let addr = state
                        .config
                        .chain_config
                        .address_from_pub_key(&signer.public_key().await?)?;
                    state
                        .distributor_addrs
                        .lock()
                        .unwrap()
                        .insert(derivation_index, addr.clone());
                    addr
                }
                Some(addr) => addr.clone(),
            };

            tracing::debug!("getting balances for {}", addr);

            let balances = state.query_client.all_balances(addr.clone(), None).await?;

            tracing::debug!("balances for {}: {:?}", addr, balances);

            if derivation_index == 0 {
                holder = Some((addr, balances));
            } else {
                distributors.insert(addr, balances);
            }
        }

        Ok(Self {
            chain_config: state.config.chain_config.clone(),
            distributors,
            holder: holder.context("holder not found")?,
        })
    }
}

#[axum::debug_handler]
pub async fn status(State(state): State<AppState>) -> impl IntoResponse {
    // These structs are JSON-friendly and backwards-compatible with CosmJS status
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct JsonStatus {
        status: &'static str,
        #[serde(rename = "chainId")]
        chain_id: String,
        #[serde(rename = "nodeUrl")]
        node_url: String,
        #[serde(rename = "chainTokens")]
        chain_tokens: Vec<String>,
        #[serde(rename = "availableTokens")]
        available_tokens: Vec<String>,
        holder: AddressWithBalance,
        distributors: Vec<AddressWithBalance>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AddressWithBalance {
        address: String,
        balances: Vec<JsonCoin>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct JsonCoin {
        denom: String,
        amount: String,
    }

    match Status::new(state.clone()).await {
        Err(e) => e.into_response(),
        Ok(status) => {
            let mut available_tokens: Vec<String> = status
                .distributors
                .values()
                .flatten()
                .map(|c| c.denom.clone())
                .collect();
            available_tokens.push(state.config.credit.denom.clone());
            available_tokens.sort();
            available_tokens.dedup();

            let status = JsonStatus {
                status: "ok",
                chain_id: status.chain_config.chain_id.to_string(),
                node_url: status.chain_config.rpc_endpoint.clone(),
                chain_tokens: vec![status.chain_config.gas_denom.clone()],
                available_tokens,
                holder: AddressWithBalance {
                    address: status.holder.0.to_string(),
                    balances: status
                        .holder
                        .1
                        .iter()
                        .map(|c| JsonCoin {
                            denom: c.denom.clone(),
                            amount: c.amount.to_string(),
                        })
                        .collect(),
                },
                distributors: status
                    .distributors
                    .iter()
                    .map(|(k, v)| AddressWithBalance {
                        address: k.to_string(),
                        balances: v
                            .iter()
                            .map(|c| JsonCoin {
                                denom: c.denom.clone(),
                                amount: c.amount.to_string(),
                            })
                            .collect(),
                    })
                    .collect(),
            };

            Json(status).into_response()
        }
    }
}

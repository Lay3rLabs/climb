use anyhow::Result;
use clap::Subcommand;
use layer_climb::{prelude::*, proto::abci::TxResponse};
use std::path::PathBuf;

#[derive(Clone, Subcommand)]
pub enum ContractCommand {
    /// Uploads a contract to the chain
    Upload {
        /// Path to the .wasm file to upload
        #[arg(long)]
        wasm_file: PathBuf,
    },

    /// Instantiates a contract on the chain
    InstantiateContract {
        /// The code ID of the contract, obtained from `upload`
        #[arg(long)]
        code_id: u64,
        /// The instantiation message, as a json-encoded string
        #[arg(long)]
        msg: Option<String>,
        /// Optional label for the contract
        #[arg(long)]
        label: Option<String>,
        /// Optional funds to send, if not set will use the chain gas denom
        #[arg(long)]
        funds_denom: Option<String>,
        /// Optional funds to send, if not set no funds will be sent
        #[arg(long)]
        funds_amount: Option<String>,
    },

    /// Executes a contract on the chain
    ExecuteContract {
        /// The address of the contract, obtained from `instantiate`
        #[arg(long)]
        address: String,
        /// The execution message, as a json-encoded string
        #[arg(long)]
        msg: Option<String>,
        /// Optional funds to send, if not set will use the chain gas denom
        #[arg(long)]
        funds_denom: Option<String>,
        /// Optional funds to send, if not set no funds will be sent
        #[arg(long)]
        funds_amount: Option<String>,
    },

    /// Queries a contract on the chain
    QueryContract {
        /// The address of the contract, obtained from `instantiate`
        #[arg(long)]
        address: String,
        /// The query message, as a json-encoded string
        #[arg(long)]
        msg: Option<String>,
    },
}

impl ContractCommand {
    pub async fn run(&self, client: impl Into<AnyClient>, log: impl Fn(ContractLog)) -> Result<()> {
        let client = client.into();
        match self {
            ContractCommand::Upload { wasm_file } => {
                let wasm_byte_code = tokio::fs::read(wasm_file).await?;
                let (code_id, tx_resp) = client
                    .as_signing()
                    .contract_upload_file(wasm_byte_code, None)
                    .await?;

                log(ContractLog::Upload {
                    code_id,
                    tx_resp: Box::new(tx_resp),
                });
            }
            ContractCommand::InstantiateContract {
                code_id,
                msg,
                label,
                funds_denom,
                funds_amount,
            } => {
                let (addr, tx_resp) = client
                    .as_signing()
                    .contract_instantiate(
                        client.as_signing().addr.clone(),
                        *code_id,
                        label.clone().unwrap_or_default(),
                        &contract_str_to_msg(msg.as_deref())?,
                        get_funds(
                            &client.as_querier().chain_config,
                            funds_denom.clone(),
                            funds_amount.clone(),
                        ),
                        None,
                    )
                    .await?;

                log(ContractLog::Instantiate {
                    addr,
                    tx_resp: Box::new(tx_resp),
                });
            }
            ContractCommand::ExecuteContract {
                address,
                msg,
                funds_denom,
                funds_amount,
            } => {
                let address = client.as_querier().chain_config.parse_address(address)?;

                let tx_resp = client
                    .as_signing()
                    .contract_execute(
                        &address,
                        &contract_str_to_msg(msg.as_deref())?,
                        get_funds(
                            &client.as_querier().chain_config,
                            funds_denom.clone(),
                            funds_amount.clone(),
                        ),
                        None,
                    )
                    .await?;

                log(ContractLog::Execute {
                    tx_resp: Box::new(tx_resp),
                });
            }
            ContractCommand::QueryContract { address, msg } => {
                let address = client.as_querier().chain_config.parse_address(address)?;

                let resp = client
                    .as_querier()
                    .contract_smart_raw(&address, &contract_str_to_msg(msg.as_deref())?)
                    .await?;
                let resp = std::str::from_utf8(&resp)?;

                log(ContractLog::Query {
                    response: resp.to_string(),
                });
            }
        }
        Ok(())
    }
}

fn get_funds(
    chain_config: &ChainConfig,
    funds_denom: Option<String>,
    funds_amount: Option<String>,
) -> Vec<Coin> {
    match funds_amount {
        Some(funds_amount) => {
            let funds_denom = funds_denom.unwrap_or(chain_config.gas_denom.clone());
            vec![new_coin(funds_denom, funds_amount)]
        }
        None => Vec::new(),
    }
}

pub enum ContractLog {
    Upload {
        code_id: u64,
        tx_resp: Box<TxResponse>,
    },
    Instantiate {
        addr: Address,
        tx_resp: Box<TxResponse>,
    },
    Execute {
        tx_resp: Box<TxResponse>,
    },
    Query {
        response: String,
    },
}

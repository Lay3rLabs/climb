#![allow(warnings)]
mod opt;

use anyhow::{anyhow, bail, Context, Result};
use bip39::Mnemonic;
use clap::Parser;
use layer_climb::prelude::*;
use opt::{Args, Command, Opt};
use rand::Rng;
use std::{fs, os::unix::net};
use tracing;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().context("couldn't find dotenv file")?;
    let args = Args::parse();

    tracing_subscriber::fmt()
        .without_time()
        .with_target(false)
        .with_max_level(tracing::Level::from(args.log_level))
        .init();

    let opt = Opt::new(args).await?;

    match opt.command.clone() {
        Command::WalletShow {} => {
            let signing_client = opt.signing_client().await?;
            tracing::info!("address: {}", signing_client.addr);
            let balances = signing_client
                .querier
                .all_balances(signing_client.addr, None)
                .await?;
            if balances.is_empty() {
                tracing::info!("No balance found");
            } else {
                tracing::info!("Balances:");
                for balance in balances {
                    tracing::info!("{}: {}", balance.denom, balance.amount);
                }
            }
        }
        Command::TapFaucet { amount } => {
            let faucet = opt.faucet_client().await?;
            let addr = opt.address().await?;
            let amount = amount.unwrap_or(1_000_000);

            tracing::info!(
                "Balance before: {}",
                faucet
                    .querier
                    .balance(addr.clone(), None)
                    .await?
                    .unwrap_or_default()
            );
            tracing::info!("Sending {} to {}", amount, addr);
            let mut tx_builder = faucet.tx_builder();
            tx_builder.set_gas_simulate_multiplier(2.0);
            faucet
                .transfer(None, amount, addr.clone(), Some(tx_builder))
                .await?;
            tracing::info!(
                "Balance after: {}",
                faucet
                    .querier
                    .balance(addr, None)
                    .await?
                    .unwrap_or_default()
            );
        }

        Command::GenerateWallet {} => {
            let mut rng = rand::thread_rng();
            let entropy: [u8; 32] = rng.gen();
            let mnemonic = Mnemonic::from_entropy(&entropy)?;

            let signer = KeySigner::new_mnemonic_iter(mnemonic.word_iter(), None)?;
            let addr = opt
                .chain_config
                .address_from_pub_key(&signer.public_key().await?)?;

            tracing::info!("--- Address ---");
            tracing::info!("{}", addr);
            tracing::info!("--- Mnemonic---");
            tracing::info!("{}", mnemonic);
        }

        Command::UploadContract { wasm_file } => {
            let wasm_byte_code = tokio::fs::read(wasm_file).await?;
            let client = opt.signing_client().await?;
            let (code_id, tx_resp) = client.contract_upload_file(wasm_byte_code, None).await?;

            tracing::info!("Tx Hash: {}", tx_resp.txhash);
            tracing::info!("Code ID: {}", code_id);
        }

        Command::InstantiateContract {
            code_id,
            msg,
            label,
            funds_denom,
            funds_amount,
        } => {
            let client = opt.signing_client().await?;

            let (addr, tx_resp) = client
                .contract_instantiate(
                    client.addr.clone(),
                    code_id,
                    label.unwrap_or_default(),
                    &contract_str_to_msg(msg.as_deref())?,
                    get_funds(&opt.chain_config, funds_denom, funds_amount),
                    None,
                )
                .await?;

            tracing::info!("Tx Hash: {}", tx_resp.txhash);
            tracing::info!("Contract Address: {}", addr);
        }

        Command::ExecuteContract {
            address,
            msg,
            funds_denom,
            funds_amount,
        } => {
            let client = opt.signing_client().await?;

            let address = opt.chain_config.parse_address(&address)?;

            let tx_resp = client
                .contract_execute(
                    &address,
                    &contract_str_to_msg(msg.as_deref())?,
                    get_funds(&opt.chain_config, funds_denom, funds_amount),
                    None,
                )
                .await?;

            tracing::info!("Tx Hash: {}", tx_resp.txhash);
        }

        Command::QueryContract { address, msg } => {
            let client = opt.signing_client().await?;

            let address = opt.chain_config.parse_address(&address)?;

            let resp = client
                .querier
                .contract_smart_raw(&address, &contract_str_to_msg(msg.as_deref())?)
                .await?;
            let resp = std::str::from_utf8(&resp)?;

            tracing::info!("Query Response: {:?}", resp);
        }
    }

    Ok(())
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

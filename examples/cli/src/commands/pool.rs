use crate::context::AppContext;
use anyhow::{anyhow, Context, Result};
/// This isn't really a very *useful* command, but it demonstrates how to use a `Pool` for concurrent transactions
/// real-world usage would be in deploy tooling (e.g. uploading/instantiating multiple contracts)
/// or bots (e.g. hitting different contracts with different transactions without worrying about sequence errors)
use clap::Subcommand;
use deadpool::managed::Pool;
use futures::future;
use layer_climb::prelude::*;

#[derive(Clone, Subcommand)]
pub enum PoolCommand {
    /// Sends funds from multiple derivations of the basic account to some other account
    /// will use the faucet to "top-up" as needed
    Multisend {
        /// The address to send the funds to
        /// if not set, will be the default client and senders will start from the same client but derivation index 1
        /// otherwise, if set, senders will start from derivation index 0
        #[arg(long)]
        to: Option<String>,
        /// The amount to send
        /// if not set, will be `Self::DEFAULT_MULTISEND_AMOUNT`
        #[arg(long)]
        amount: Option<u128>,
        /// The denom of the funds to send, if not set will use the chain gas denom
        #[arg(long)]
        denom: Option<String>,

        /// number of sends
        /// if not set, will be 2x the number of concurrent accounts
        #[arg(long)]
        sends: Option<usize>,

        /// The number of concurrent accounts to use
        /// if not set, will be `Self::DEFAULT_MAX_CONCURRENT_ACCOUNTS`
        #[arg(long)]
        max_concurrent_accounts: Option<usize>,

        /// The minimum balance to maintain
        /// set this to as low as reasonable, to reduce unnecessary transfers
        /// if not set, will be `Self::DEFAULT_MINIMUM_BALANCE_THRESHHOLD`
        #[arg(long)]
        minimum_balance_threshhold: Option<u128>,

        /// The amount to send to top up the account
        /// set this to as high as reasonable, to reduce unnecessary transfers
        /// if not set, will be `Self::DEFAULT_MINIMUM_BALANCE_TOPUP_AMOUNT`
        #[arg(long)]
        minimum_balance_topup_amount: Option<u128>,
    },
}

impl PoolCommand {
    const DEFAULT_MULTISEND_AMOUNT: u128 = 2_000;
    const DEFAULT_MAX_CONCURRENT_ACCOUNTS: usize = 3;
    const DEFAULT_MINIMUM_BALANCE_THRESHHOLD: u128 = 20_000;
    const DEFAULT_MINIMUM_BALANCE_TOPUP_AMOUNT: u128 = 1_000_000;

    pub async fn run(&self, ctx: &AppContext) -> Result<()> {
        match self {
            PoolCommand::Multisend {
                to,
                amount,
                denom,
                max_concurrent_accounts,
                minimum_balance_threshhold,
                minimum_balance_topup_amount,
                sends,
            } => {
                let start_derivation_index = match to {
                    // if we're sending to some other address, we should start our senders from derivation index 0
                    Some(_) => Some(0),
                    // if we're sending to the default client, we should start our senders from derivation index 1
                    None => Some(1),
                };

                let to = match to {
                    Some(to) => ctx.chain_config()?.parse_address(to)?,
                    None => ctx.any_client().await?.as_signing().addr.clone(),
                };

                let amount = amount.unwrap_or(Self::DEFAULT_MULTISEND_AMOUNT);
                let max_concurrent_accounts =
                    max_concurrent_accounts.unwrap_or(Self::DEFAULT_MAX_CONCURRENT_ACCOUNTS);
                let minimum_balance_threshhold =
                    minimum_balance_threshhold.unwrap_or(Self::DEFAULT_MINIMUM_BALANCE_THRESHHOLD);
                let minimum_balance_send_amount = minimum_balance_topup_amount
                    .unwrap_or(Self::DEFAULT_MINIMUM_BALANCE_TOPUP_AMOUNT);

                let client_pool_manager = SigningClientPoolManager::new_mnemonic(
                    ctx.client_mnemonic()?,
                    ctx.chain_config()?,
                    start_derivation_index,
                )
                .with_minimum_balance(
                    minimum_balance_threshhold,
                    minimum_balance_send_amount,
                    Some(ctx.create_faucet().await?),
                    denom.clone(),
                )
                .await?;

                let client_pool: Pool<SigningClientPoolManager> =
                    Pool::builder(client_pool_manager)
                        .max_size(max_concurrent_accounts)
                        .build()
                        .context("Failed to create client pool")?;

                let sends = match sends {
                    None => max_concurrent_accounts * 2,
                    Some(sends) => *sends,
                };

                let mut futures = Vec::new();
                for _ in 0..sends {
                    let to = to.clone();
                    let denom = denom.clone();
                    let client_pool = client_pool.clone();
                    futures.push(async move {
                        let client = client_pool.get().await.map_err(|e| anyhow!("{e:?}"))?;
                        let _ = client.transfer(amount, &to, denom.as_deref(), None).await?;
                        tracing::info!("Sent {} to {} from {}", amount, to, client.addr);
                        anyhow::Ok(())
                    });
                }

                tracing::info!(
                    "Balance before: {}",
                    ctx.chain_querier()
                        .await?
                        .balance(to.clone(), denom.clone())
                        .await?
                        .unwrap_or_default()
                );

                future::try_join_all(futures).await?;

                tracing::info!(
                    "Balance after: {}",
                    ctx.chain_querier()
                        .await?
                        .balance(to.clone(), denom.clone())
                        .await?
                        .unwrap_or_default()
                );
            }
        }

        Ok(())
    }
}

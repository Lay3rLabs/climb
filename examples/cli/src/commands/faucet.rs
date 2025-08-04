// This uses some designated account as a "faucet" to send tokens to other accounts.
// the faucet mnemonic is hardcoded in the config

use crate::context::AppContext;
use anyhow::Result;
use clap::Subcommand;

#[derive(Clone, Subcommand)]
pub enum FaucetCommand {
    /// Tap the faucet to get some funds
    Tap {
        /// The address to send the funds to
        /// if not set, will be the default client
        #[arg(long)]
        to: Option<String>,
        /// The amount to send
        /// if not set, will be `Self::DEFAULT_TAP_AMOUNT`
        #[arg(long)]
        amount: Option<u128>,
        /// The denom of the funds to send, if not set will use the chain gas denom
        #[arg(long)]
        denom: Option<String>,
    },
}

impl FaucetCommand {
    const DEFAULT_TAP_AMOUNT: u128 = 1_000_000;

    pub async fn run(&self, ctx: &AppContext) -> Result<()> {
        match self {
            FaucetCommand::Tap { to, amount, denom } => {
                let to = match to {
                    Some(to) => ctx.chain_config()?.parse_address(to)?,
                    None => ctx.any_client().await?.as_signing().addr.clone(),
                };

                let amount = amount.unwrap_or(Self::DEFAULT_TAP_AMOUNT);
                let faucet = ctx.create_faucet().await?;

                tracing::info!(
                    "Balance before: {}",
                    ctx.chain_querier()
                        .await?
                        .balance(to.clone(), denom.clone())
                        .await?
                        .unwrap_or_default()
                );

                let tx_resp = faucet.transfer(amount, &to, denom.as_deref(), None).await?;

                tracing::info!("Tapped faucet for {}, sent to {}", amount, to);
                tracing::info!(
                    "Balance after: {}",
                    ctx.chain_querier()
                        .await?
                        .balance(to.clone(), denom.clone())
                        .await?
                        .unwrap_or_default()
                );
                tracing::info!("Tx hash: {:?}", tx_resp.txhash);
            }
        }
        Ok(())
    }
}

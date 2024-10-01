use anyhow::Result;
use bip39::Mnemonic;
use clap::Subcommand;
use layer_climb::{prelude::*, proto::abci::TxResponse};
use rand::Rng;

#[derive(Clone, Subcommand)]
pub enum WalletCommand {
    /// Creates a wallet with a random mnemonic
    Create,
    /// Shows the current signing wallet balance and address
    Show,
    /// Shows the balances for a given address
    Balance {
        #[arg(long)]
        /// The address to show the balance for
        address: String,
        /// Denom to show the balance for, if not set will default to the chain's gas denom
        #[arg(long)]
        denom: Option<String>,
    },
    AllBalances {
        #[arg(long)]
        /// The address to show the balances for
        address: String,
    },
    /// Transfer funds to another address
    Transfer {
        #[arg(long)]
        /// The address to send the funds to
        to: String,
        /// The amount to send
        #[arg(long)]
        amount: u128,
        /// The denom of the funds to send, if not set will use the chain gas denom
        #[arg(long)]
        denom: Option<String>,
    },
}

impl WalletCommand {
    pub async fn run(
        &self,
        client: impl Into<AnyClient>,
        rng: &mut impl Rng,
        log: impl Fn(WalletLog),
    ) -> Result<()> {
        let client = client.into();
        match self {
            WalletCommand::Create => {
                let entropy: [u8; 32] = rng.gen();
                let mnemonic = Mnemonic::from_entropy(&entropy)?;

                let signer = KeySigner::new_mnemonic_iter(mnemonic.word_iter(), None)?;
                let addr = client
                    .as_querier()
                    .chain_config
                    .address_from_pub_key(&signer.public_key().await?)?;

                log(WalletLog::Create {
                    addr: addr.clone(),
                    mnemonic: mnemonic.clone(),
                });
            }
            WalletCommand::Show => {
                let balances = client
                    .as_querier()
                    .all_balances(client.as_signing().addr.clone(), None)
                    .await?;

                if balances.is_empty() {
                    log(WalletLog::Show {
                        addr: client.as_signing().addr.clone(),
                        balances: vec![],
                    });
                } else {
                    log(WalletLog::Show {
                        addr: client.as_signing().addr.clone(),
                        balances: balances.clone(),
                    });
                }
            }
            WalletCommand::Balance { address, denom } => {
                let addr = client.as_querier().chain_config.parse_address(address)?;
                let balance = client
                    .as_querier()
                    .balance(addr.clone(), denom.clone())
                    .await?;
                let denom = denom
                    .clone()
                    .unwrap_or_else(|| client.as_querier().chain_config.gas_denom.clone());
                log(WalletLog::Balance {
                    addr,
                    balance: new_coin(balance.unwrap_or_default(), denom),
                });
            }
            WalletCommand::AllBalances { address } => {
                let addr = client.as_querier().chain_config.parse_address(address)?;
                let balances = client.as_querier().all_balances(addr.clone(), None).await?;
                log(WalletLog::AllBalances { addr, balances });
            }
            WalletCommand::Transfer { to, amount, denom } => {
                let to = client.as_querier().chain_config.parse_address(to)?;
                let tx_resp = client
                    .as_signing()
                    .transfer(*amount, &to, denom.as_deref(), None)
                    .await?;
                log(WalletLog::Transfer {
                    to,
                    amount: *amount,
                    denom: denom
                        .clone()
                        .unwrap_or_else(|| client.as_querier().chain_config.gas_denom.clone()),
                    tx_resp: Box::new(tx_resp),
                });
            }
        }
        Ok(())
    }
}

pub enum WalletLog {
    Create {
        addr: Address,
        mnemonic: Mnemonic,
    },
    Show {
        addr: Address,
        balances: Vec<Coin>,
    },
    Balance {
        addr: Address,
        balance: Coin,
    },
    AllBalances {
        addr: Address,
        balances: Vec<Coin>,
    },
    Transfer {
        to: Address,
        amount: u128,
        denom: String,
        tx_resp: Box<TxResponse>,
    },
}

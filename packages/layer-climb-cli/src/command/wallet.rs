use anyhow::Result;
use bip39::Mnemonic;
use clap::Subcommand;
use layer_climb::{prelude::*, proto::abci::TxResponse};
use rand::Rng;

#[derive(Debug, Clone, Subcommand)]
pub enum WalletCommand {
    /// Creates a wallet with a random mnemonic
    Create,
    /// Shows the balance and address for a given mnemonic
    /// If no mnemonic is provided, the default client mnemonic will be used
    Show {
        #[arg(long)]
        mnemonic: Option<String>,
    },
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
                let (addr, mnemonic) =
                    create_wallet(client.as_querier().chain_config.clone(), rng).await?;
                log(WalletLog::Create { addr, mnemonic });
            }
            WalletCommand::Show { mnemonic } => {
                let addr = match mnemonic {
                    None => client.as_signing().addr.clone(),
                    Some(mnemonic) => {
                        let signer = KeySigner::new_mnemonic_str(mnemonic, None)?;
                        client
                            .as_querier()
                            .chain_config
                            .address_from_pub_key(&signer.public_key().await?)?
                    }
                };

                let balances = client.as_querier().all_balances(addr.clone(), None).await?;

                if balances.is_empty() {
                    log(WalletLog::Show {
                        addr,
                        balances: vec![],
                    });
                } else {
                    log(WalletLog::Show {
                        addr,
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

pub async fn create_wallet(
    chain_config: ChainConfig,
    rng: &mut impl Rng,
) -> Result<(Address, Mnemonic)> {
    let entropy: [u8; 32] = rng.random();
    let mnemonic = Mnemonic::from_entropy(&entropy)?;

    let signer = KeySigner::new_mnemonic_iter(mnemonic.words(), None)?;
    let addr = chain_config.address_from_pub_key(&signer.public_key().await?)?;

    Ok((addr, mnemonic))
}

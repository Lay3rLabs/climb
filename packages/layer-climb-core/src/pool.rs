use std::sync::atomic::AtomicU32;

use crate::{
    cache::ClimbCache,
    querier::{Connection, QueryClient},
    signing::SigningClient,
};
use anyhow::{bail, Error, Result};
use deadpool::managed::{Manager, Metrics, Object, PoolError, RecycleResult};
use layer_climb_address::*;
use layer_climb_config::ChainConfig;
use tokio::sync::Mutex;

/// Currently this only works with mnemonic phrases
pub struct SigningClientPoolManager {
    pub mnemonic: String,
    pub derivation_index: AtomicU32,
    pub chain_config: ChainConfig,
    pub balance_maintainer: Option<BalanceMaintainer>,
    pub cache: ClimbCache,
    pub connection: Connection,
}

impl SigningClientPoolManager {
    pub fn new_mnemonic(
        mnemonic: String,
        chain_config: ChainConfig,
        start_index: Option<u32>,
        connection: Option<Connection>,
    ) -> Self {
        let connection = connection.unwrap_or_default();
        Self {
            mnemonic,
            chain_config,
            derivation_index: AtomicU32::new(start_index.unwrap_or_default()),
            balance_maintainer: None,
            cache: ClimbCache::new(connection.rpc.clone()),
            connection,
        }
    }

    // Setting this has a few implications:
    // 1. on each client hand-out, it will query for the balance (no locking at all, just another query)
    // 2. if the balance is below the threshhold set here, then it will lock the funding client for the transfer
    //
    // in other words, while the pool itself is completely async and can be parallelized, the balance maintainer
    // does crate an async await across all clients who need to top-up an account, if they happen at the same time
    //
    // This isn't a major performance impact, but nevertheless,
    // it's recommended to tune the values so that it's reasonably infrequent
    pub async fn with_minimum_balance(
        mut self,
        // the minimum balance to maintain
        // set this to as low as reasonable, to reduce unnecessary transfers
        threshhold: u128,
        // the amount to send to top up the account
        // set this to as high as reasonable, to reduce unnecessary transfers
        amount: u128,
        // if set, it will use this client to fund the account
        // otherwise, it will use the first derivation index, and bump it for subsequent clients
        funder: Option<SigningClient>,
        denom: Option<String>,
    ) -> Result<Self> {
        // keep a separate query client so we can get balances
        // without locking the funder client
        let query_client =
            QueryClient::new(self.chain_config.clone(), Some(self.connection.clone())).await?;

        let balance_maintainer = match funder {
            Some(funder) => BalanceMaintainer {
                client: Mutex::new(funder),
                query_client,
                threshhold,
                amount,
                denom,
            },
            None => BalanceMaintainer {
                client: Mutex::new(self.create_client(None).await?),
                query_client,
                threshhold,
                amount,
                denom,
            },
        };

        self.balance_maintainer = Some(balance_maintainer);
        Ok(self)
    }

    fn create_signer(&self) -> Result<KeySigner> {
        match &self.chain_config.address_kind {
            layer_climb_config::AddrKind::Cosmos { .. } => {
                let index = self
                    .derivation_index
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                KeySigner::new_mnemonic_str(&self.mnemonic, Some(&cosmos_hub_derivation(index)?))
            }
            layer_climb_config::AddrKind::Evm => {
                bail!("EVM address kind is not supported (yet)")
            }
        }
    }

    async fn create_client(&self, signer: Option<KeySigner>) -> Result<SigningClient> {
        let signer: KeySigner = match signer {
            Some(signer) => signer,
            None => self.create_signer()?,
        };

        SigningClient::new_with_cache(
            self.chain_config.clone(),
            signer,
            self.cache.clone(),
            Some(self.connection.clone()),
        )
        .await
    }

    async fn maybe_top_up(&self, addr: Address) -> Result<()> {
        if let Some(balance_maintainer) = &self.balance_maintainer {
            let current_balance = balance_maintainer
                .query_client
                .balance(addr.clone(), balance_maintainer.denom.clone())
                .await?
                .unwrap_or_default();
            if current_balance < balance_maintainer.threshhold {
                let amount = balance_maintainer.amount - current_balance;
                // just a scope to ensure we always drop the lock
                {
                    let funder = balance_maintainer.client.lock().await;

                    tracing::debug!(
                        "Balance on {} is {}, below {}, sending {} to top-up from {}",
                        addr,
                        current_balance,
                        balance_maintainer.threshhold,
                        amount,
                        funder.addr
                    );

                    funder
                        .transfer(amount, &addr, balance_maintainer.denom.as_deref(), None)
                        .await?;
                }
            }
        }

        Ok(())
    }
}

// just a helper struct to keep track of the balance maintainer
pub struct BalanceMaintainer {
    pub client: Mutex<SigningClient>,
    query_client: QueryClient,
    threshhold: u128,
    amount: u128,
    denom: Option<String>,
}

impl Manager for SigningClientPoolManager {
    type Type = SigningClient;
    type Error = anyhow::Error;

    async fn create(&self) -> Result<SigningClient> {
        // it's possible that the client hasn't ever been funded
        // which would cause an error when trying to create it (specifically in base account)
        // so if we're configured to use a funder, let's get the raw address
        // before we create the client
        let signer = self.create_signer()?;
        let addr = self
            .chain_config
            .address_from_pub_key(&signer.public_key().await?)?;

        self.maybe_top_up(addr).await?;
        let client = self.create_client(Some(signer)).await?;

        tracing::debug!("POOL CREATED CLIENT {}", client.addr);

        Ok(client)
    }

    async fn recycle(
        &self,
        client: &mut SigningClient,
        _: &Metrics,
    ) -> RecycleResult<anyhow::Error> {
        tracing::debug!("POOL RECYCLING CLIENT {}", client.addr);

        Ok(())
    }
}

#[derive(Clone)]
pub struct SigningClientPool {
    pub pool: deadpool::managed::Pool<SigningClientPoolManager>,
}

impl SigningClientPool {
    pub fn new(pool: deadpool::managed::Pool<SigningClientPoolManager>) -> Self {
        Self { pool }
    }

    pub async fn get(&self) -> Result<Object<SigningClientPoolManager>, PoolError<Error>> {
        let client = self.pool.get().await?;
        self.pool
            .manager()
            .maybe_top_up(client.addr.clone())
            .await?;
        Ok(client)
    }
}

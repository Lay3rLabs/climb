use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::config::Config;
use anyhow::Result;
use deadpool::managed::Pool;
use layer_climb::{pool::SigningClientPoolManager, prelude::*};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub client_pool: SigningClientPool,
    pub query_client: QueryClient,
    pub distributor_addrs: Arc<Mutex<HashMap<u32, Address>>>,
}

impl AppState {
    // Getting a context requires parsing the args first
    pub async fn new(config: Config) -> Result<Self> {
        let client_pool_manager = SigningClientPoolManager::new_mnemonic(
            config.mnemonic.clone(),
            config.chain_config.clone(),
            None,
            None,
        )
        .with_minimum_balance(
            config.minimum_credit_balance_threshhold,
            config.minimum_credit_balance_topup,
            None,
            Some(config.credit.denom.clone()),
        )
        .await?;

        let client_pool = SigningClientPool::new(
            Pool::builder(client_pool_manager)
                .max_size(config.concurrency)
                .build()?,
        );

        let query_client = QueryClient::new(config.chain_config.clone(), None).await?;

        Ok(Self {
            config: Arc::new(config),
            client_pool,
            query_client,
            distributor_addrs: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

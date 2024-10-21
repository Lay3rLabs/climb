pub mod abci;
pub mod basic;
pub mod contract;
pub mod fetch;
pub mod ibc;
pub mod middleware;
pub mod stream;
pub mod tx;
pub mod validator;

use std::{
    future::Future,
    sync::{atomic::AtomicU8, Arc},
    time::Duration,
};

use middleware::{QueryMiddlewareMapReq, QueryMiddlewareMapResp, QueryMiddlewareRun};

use crate::{cache::ClimbCache, network::rpc::RpcClient, prelude::*};

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[derive(Clone)]
        pub struct QueryClient {
            pub chain_config: ChainConfig,
            pub cache: ClimbCache,
            pub grpc_channel: tonic_web_wasm_client::Client,
            pub rpc_client: RpcClient,
            pub middleware_map_req: Arc<Vec<QueryMiddlewareMapReq>>,
            pub middleware_map_resp: Arc<Vec<QueryMiddlewareMapResp>>,
            pub middleware_run: Arc<Vec<QueryMiddlewareRun>>,
            pub balances_pagination_limit: u64,
            pub wait_blocks_poll_sleep_duration: Duration,
            _abci_query_mode: Arc<AtomicU8>,
        }
    } else {
        #[derive(Clone)]
        pub struct QueryClient {
            pub chain_config: ChainConfig,
            pub cache: ClimbCache,
            pub grpc_channel: tonic::transport::Channel,
            pub rpc_client: RpcClient,
            pub middleware_map_req: Arc<Vec<QueryMiddlewareMapReq>>,
            pub middleware_map_resp: Arc<Vec<QueryMiddlewareMapResp>>,
            pub middleware_run: Arc<Vec<QueryMiddlewareRun>>,
            pub balances_pagination_limit: u64,
            pub wait_blocks_poll_sleep_duration: Duration,
            _abci_query_mode: Arc<AtomicU8>,
        }
    }
}

pub trait QueryRequest: Clone + std::fmt::Debug + Send {
    type QueryResponse: std::fmt::Debug + Send;

    fn request(&self, client: QueryClient) -> impl Future<Output = Result<Self::QueryResponse>>;
}

const DEFAULT_BALANCES_PAGINATION_LIMIT: u64 = 10;
const DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION: std::time::Duration =
    std::time::Duration::from_secs(1);

impl QueryClient {
    pub async fn new(chain_config: ChainConfig) -> Result<Self> {
        let cache = ClimbCache::default();
        Self::new_with_cache(chain_config, cache).await
    }

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            pub async fn new_with_cache(chain_config: ChainConfig, cache: ClimbCache) -> Result<Self> {
                let grpc_channel = cache.get_grpc(&chain_config).await?;
                let rpc_client = cache.get_rpc_client(&chain_config.rpc_endpoint);

                let _self = Self {
                    chain_config,
                    cache,
                    grpc_channel,
                    rpc_client,
                    middleware_map_req: Arc::new(QueryMiddlewareMapReq::default_list()),
                    middleware_map_resp: Arc::new(QueryMiddlewareMapResp::default_list()),
                    middleware_run: Arc::new(QueryMiddlewareRun::default_list()),
                    balances_pagination_limit: DEFAULT_BALANCES_PAGINATION_LIMIT,
                    wait_blocks_poll_sleep_duration: DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION,
                    _abci_query_mode: Arc::new(AtomicU8::new(QueryClientMode::Grpc as u8))
                };

                _self.set_abci_query_client_mode(None).await?;

                Ok(_self)
            }
        } else {
            pub async fn new_with_cache(chain_config: ChainConfig, cache: ClimbCache) -> Result<Self> {
                let grpc_channel = cache.get_grpc(&chain_config).await?;
                let rpc_client = cache.get_rpc_client(&chain_config.rpc_endpoint);

                let _self = Self {
                    chain_config,
                    cache,
                    grpc_channel,
                    rpc_client,
                    middleware_map_req: Arc::new(QueryMiddlewareMapReq::default_list()),
                    middleware_map_resp: Arc::new(QueryMiddlewareMapResp::default_list()),
                    middleware_run: Arc::new(QueryMiddlewareRun::default_list()),
                    balances_pagination_limit: DEFAULT_BALANCES_PAGINATION_LIMIT,
                    wait_blocks_poll_sleep_duration: DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION,
                    _abci_query_mode: Arc::new(AtomicU8::new(QueryClientMode::Grpc as u8))
                };

                _self.set_abci_query_client_mode(None).await?;

                Ok(_self)
            }
        }
    }

    pub async fn run_with_middleware<REQ: QueryRequest>(
        &self,
        mut req: REQ,
    ) -> Result<REQ::QueryResponse> {
        for middleware in self.middleware_map_req.iter() {
            req = match middleware.map_req(req.clone()).await {
                Ok(req) => req,
                Err(e) => return Err(e),
            }
        }

        let mut response = None;

        for middleware in self.middleware_run.iter() {
            response = match middleware.run(req.clone(), self.clone()).await {
                Ok(resp) => Some(resp),
                Err(e) => return Err(e),
            }
        }

        if response.is_none() {
            response = Some(req.request(self.clone()).await?);
        }

        let mut response = response.unwrap();

        for middleware in self.middleware_map_resp.iter() {
            response = match middleware.map_resp(response).await {
                Ok(resp) => resp,
                Err(e) => return Err(e),
            }
        }

        Ok(response)
    }

    // these do not call middleware, but their inner calls do
    pub async fn wait_until_block_height(
        &self,
        target_block_height: u64,
        sleep_duration: Option<Duration>,
    ) -> Result<()> {
        let sleep_duration = sleep_duration.unwrap_or(self.wait_blocks_poll_sleep_duration);
        loop {
            let current_block_height = self.block_height().await?;

            if current_block_height >= target_block_height {
                break Ok(());
            }

            futures_timer::Delay::new(sleep_duration).await;
        }
    }

    pub async fn wait_blocks(&self, n_blocks: u64, sleep_duration: Option<Duration>) -> Result<()> {
        let target_block_height = self.block_height().await? + n_blocks;
        self.wait_until_block_height(target_block_height, sleep_duration)
            .await
    }
}

// currently only used via automatic fallback in very specific cases
// TODO: make this more general
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum QueryClientMode {
    Grpc,
    Rpc,
}

impl From<QueryClientMode> for u8 {
    fn from(mode: QueryClientMode) -> u8 {
        mode as u8
    }
}

impl From<u8> for QueryClientMode {
    fn from(mode: u8) -> QueryClientMode {
        match mode {
            0 => QueryClientMode::Grpc,
            1 => QueryClientMode::Rpc,
            _ => panic!("invalid QueryClientMode"),
        }
    }
}

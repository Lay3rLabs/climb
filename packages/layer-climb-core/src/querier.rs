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
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
    time::Duration,
};

use abci::{AbciProofKind, AbciProofReq};
use middleware::{QueryMiddlewareMapReq, QueryMiddlewareMapResp, QueryMiddlewareRun};
use tracing::instrument;

use crate::{cache::ClimbCache, network::rpc::RpcClient, prelude::*};

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[derive(Clone)]
        pub struct QueryClient {
            pub chain_config: ChainConfig,
            pub cache: ClimbCache,
            pub middleware_map_req: Arc<Vec<QueryMiddlewareMapReq>>,
            pub middleware_map_resp: Arc<Vec<QueryMiddlewareMapResp>>,
            pub middleware_run: Arc<Vec<QueryMiddlewareRun>>,
            pub balances_pagination_limit: u64,
            pub wait_blocks_poll_sleep_duration: Duration,
            _grpc_channel: Option<tonic_web_wasm_client::Client>,
            _rpc_client: Option<RpcClient>,
            _connection_mode: Arc<AtomicU8>,
        }

        impl QueryClient {
            pub fn clone_grpc_channel(&self) -> Result<tonic_web_wasm_client::Client> {
                match self._grpc_channel.clone() {
                    Some(channel) => Ok(channel),
                    None => Err(anyhow!("grpc_channel isn't set")),
                }
            }
        }
    } else {
        #[derive(Clone)]
        pub struct QueryClient {
            pub chain_config: ChainConfig,
            pub cache: ClimbCache,
            pub middleware_map_req: Arc<Vec<QueryMiddlewareMapReq>>,
            pub middleware_map_resp: Arc<Vec<QueryMiddlewareMapResp>>,
            pub middleware_run: Arc<Vec<QueryMiddlewareRun>>,
            pub balances_pagination_limit: u64,
            pub wait_blocks_poll_sleep_duration: Duration,
            _grpc_channel: Option<tonic::transport::Channel>,
            _rpc_client: Option<RpcClient>,
            _connection_mode: Arc<AtomicU8>,
        }

        impl QueryClient {
            pub fn clone_grpc_channel(&self) -> Result<tonic::transport::Channel> {
                match self._grpc_channel.clone() {
                    Some(channel) => Ok(channel),
                    None => Err(anyhow!("grpc_channel isn't set")),
                }
            }
        }
    }
}

impl std::fmt::Debug for QueryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QueryClient")
            .field("chain_id", &self.chain_config.chain_id)
            .finish()
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
    pub async fn new(
        chain_config: ChainConfig,
        default_connection_mode: Option<ConnectionMode>,
    ) -> Result<Self> {
        let cache = ClimbCache::default();
        Self::new_with_cache(chain_config, cache, default_connection_mode).await
    }

    // if None, will make a best-guess attempt via abci query
    #[instrument]
    pub async fn set_connection_mode(&self, mode: Option<ConnectionMode>) -> Result<()> {
        match mode {
            Some(mode) => {
                self._connection_mode.store(mode.into(), Ordering::SeqCst);
            }
            None => {
                let modes = vec![ConnectionMode::Grpc, ConnectionMode::Rpc];

                for mode in modes {
                    self._connection_mode.store(mode.into(), Ordering::SeqCst);

                    let is_valid = AbciProofReq {
                        kind: AbciProofKind::StakingParams,
                        height: None,
                    }
                    .request(self.clone())
                    .await
                    .is_ok();

                    if is_valid {
                        break;
                    }
                }
            }
        };

        Ok(())
    }

    pub fn get_connection_mode(&self) -> ConnectionMode {
        self._connection_mode.load(Ordering::SeqCst).into()
    }

    pub fn rpc_client(&self) -> Result<&RpcClient> {
        match self._rpc_client.as_ref() {
            Some(client) => Ok(client),
            None => Err(anyhow!("rpc_client isn't set")),
        }
    }

    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            pub async fn new_with_cache(chain_config: ChainConfig, cache: ClimbCache, default_connection_mode: Option<ConnectionMode>) -> Result<Self> {
                let default_connection_mode = default_connection_mode.unwrap_or(ConnectionMode::Grpc);

                let _grpc_channel = cache.get_grpc(&chain_config).await?;
                let _rpc_client = cache.get_rpc_client(&chain_config);


                let _self = Self {
                    chain_config,
                    cache,
                    middleware_map_req: Arc::new(QueryMiddlewareMapReq::default_list()),
                    middleware_map_resp: Arc::new(QueryMiddlewareMapResp::default_list()),
                    middleware_run: Arc::new(QueryMiddlewareRun::default_list()),
                    balances_pagination_limit: DEFAULT_BALANCES_PAGINATION_LIMIT,
                    wait_blocks_poll_sleep_duration: DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION,
                    _grpc_channel,
                    _rpc_client,
                    _connection_mode: Arc::new(AtomicU8::new(default_connection_mode as u8))
                };

                Ok(_self)
            }
        } else {
            pub async fn new_with_cache(chain_config: ChainConfig, cache: ClimbCache, default_connection_mode: Option<ConnectionMode>) -> Result<Self> {
                let default_connection_mode = default_connection_mode.unwrap_or(ConnectionMode::Grpc);

                let _grpc_channel = cache.get_grpc(&chain_config).await?;
                let _rpc_client = cache.get_rpc_client(&chain_config);


                let _self = Self {
                    chain_config,
                    cache,
                    middleware_map_req: Arc::new(QueryMiddlewareMapReq::default_list()),
                    middleware_map_resp: Arc::new(QueryMiddlewareMapResp::default_list()),
                    middleware_run: Arc::new(QueryMiddlewareRun::default_list()),
                    balances_pagination_limit: DEFAULT_BALANCES_PAGINATION_LIMIT,
                    wait_blocks_poll_sleep_duration: DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION,
                    _grpc_channel,
                    _rpc_client,
                    _connection_mode: Arc::new(AtomicU8::new(default_connection_mode as u8))
                };

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
pub enum ConnectionMode {
    Grpc,
    Rpc,
}

impl From<ConnectionMode> for u8 {
    fn from(mode: ConnectionMode) -> u8 {
        mode as u8
    }
}

impl From<u8> for ConnectionMode {
    fn from(mode: u8) -> ConnectionMode {
        match mode {
            0 => ConnectionMode::Grpc,
            1 => ConnectionMode::Rpc,
            _ => panic!("invalid ConnectionMode"),
        }
    }
}

impl std::fmt::Display for ConnectionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionMode::Grpc => write!(f, "grpc"),
            ConnectionMode::Rpc => write!(f, "rpc"),
        }
    }
}

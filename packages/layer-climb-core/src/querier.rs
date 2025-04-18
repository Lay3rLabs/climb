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

use basic::BlockHeightReq;
use middleware::{QueryMiddlewareMapReq, QueryMiddlewareMapResp, QueryMiddlewareRun};
use tracing::instrument;

use crate::{
    cache::ClimbCache,
    network::rpc::{RpcClient, RpcTransport},
    prelude::*,
};

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", target_os = "unknown"))] {
        #[derive(Clone)]
        pub struct QueryClient {
            pub chain_config: ChainConfig,
            pub cache: ClimbCache,
            pub middleware_map_req: Arc<Vec<QueryMiddlewareMapReq>>,
            pub middleware_map_resp: Arc<Vec<QueryMiddlewareMapResp>>,
            pub middleware_run: Arc<Vec<QueryMiddlewareRun>>,
            pub balances_pagination_limit: u64,
            pub wait_blocks_poll_sleep_duration: Duration,
            pub connection: Connection,
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
    } else if #[cfg(target_arch = "wasm32")] {
        #[derive(Clone)]
        pub struct QueryClient {
            pub chain_config: ChainConfig,
            pub cache: ClimbCache,
            pub middleware_map_req: Arc<Vec<QueryMiddlewareMapReq>>,
            pub middleware_map_resp: Arc<Vec<QueryMiddlewareMapResp>>,
            pub middleware_run: Arc<Vec<QueryMiddlewareRun>>,
            pub balances_pagination_limit: u64,
            pub wait_blocks_poll_sleep_duration: Duration,
            pub connection: Connection,
            _rpc_client: Option<RpcClient>,
            _connection_mode: Arc<AtomicU8>,
        }

        impl QueryClient {
            pub fn clone_grpc_channel(&self) -> Result<crate::network::grpc_wasi::Client> {
                Err(anyhow!("todo!"))
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
            pub connection: Connection,
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
    pub async fn new(chain_config: ChainConfig, connection: Option<Connection>) -> Result<Self> {
        let connection = connection.unwrap_or_default();
        let cache = ClimbCache::new(connection.rpc.clone());
        Self::new_with_cache(chain_config, cache, Some(connection)).await
    }

    // if None, will make a best-guess attempt via block query
    #[instrument]
    pub async fn set_connection_mode(&self, mode: Option<ConnectionMode>) -> Result<()> {
        match mode {
            Some(mode) => {
                self._connection_mode.store(mode.into(), Ordering::SeqCst);
            }
            None => {
                for mode in ConnectionMode::modes_to_try() {
                    self._connection_mode.store(mode.into(), Ordering::SeqCst);

                    let block_height = BlockHeightReq {}.request(self.clone()).await;

                    if let Ok(block_height) = block_height {
                        if block_height > 0 {
                            break;
                        }
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
        if #[cfg(all(target_arch = "wasm32", target_os = "unknown"))] {
            pub async fn new_with_cache(chain_config: ChainConfig, cache: ClimbCache, connection: Option<Connection>) -> Result<Self> {
                let _grpc_channel = cache.get_web_grpc(&chain_config).await?;
                let _rpc_client = cache.get_rpc_client(&chain_config);

                let connection = connection.unwrap_or_default();

                let _self = Self {
                    // if None, this will be overriden, just set _something_
                    _connection_mode: Arc::new(AtomicU8::new(connection.preferred_mode.unwrap_or(ConnectionMode::Grpc) as u8)),
                    chain_config,
                    cache,
                    middleware_map_req: Arc::new(QueryMiddlewareMapReq::default_list()),
                    middleware_map_resp: Arc::new(QueryMiddlewareMapResp::default_list()),
                    middleware_run: Arc::new(QueryMiddlewareRun::default_list()),
                    balances_pagination_limit: DEFAULT_BALANCES_PAGINATION_LIMIT,
                    wait_blocks_poll_sleep_duration: DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION,
                    _grpc_channel,
                    _rpc_client,
                    connection,
                };

                if _self.connection.preferred_mode.is_none() {
                    _self.set_connection_mode(None).await?;
                }

                Ok(_self)
            }
        } else if #[cfg(target_arch = "wasm32")] {
            pub async fn new_with_cache(chain_config: ChainConfig, cache: ClimbCache, connection: Option<Connection>) -> Result<Self> {
                let _rpc_client = cache.get_rpc_client(&chain_config);

                let connection = connection.unwrap_or_default();

                let _self = Self {
                    // if None, this will be overriden, just set _something_
                    _connection_mode: Arc::new(AtomicU8::new(connection.preferred_mode.unwrap_or(ConnectionMode::Rpc) as u8)),
                    chain_config,
                    cache,
                    middleware_map_req: Arc::new(QueryMiddlewareMapReq::default_list()),
                    middleware_map_resp: Arc::new(QueryMiddlewareMapResp::default_list()),
                    middleware_run: Arc::new(QueryMiddlewareRun::default_list()),
                    balances_pagination_limit: DEFAULT_BALANCES_PAGINATION_LIMIT,
                    wait_blocks_poll_sleep_duration: DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION,
                    _rpc_client,
                    connection,
                };

                if _self.connection.preferred_mode.is_none() {
                    _self.set_connection_mode(None).await?;
                }

                Ok(_self)
            }
        } else {
            pub async fn new_with_cache(chain_config: ChainConfig, cache: ClimbCache, connection: Option<Connection>) -> Result<Self> {
                let _grpc_channel = cache.get_grpc(&chain_config).await?;
                let _rpc_client = cache.get_rpc_client(&chain_config);

                let connection = connection.unwrap_or_default();

                let _self = Self {
                    // if None, this will be overriden, just set _something_
                    _connection_mode: Arc::new(AtomicU8::new(connection.preferred_mode.unwrap_or(ConnectionMode::Rpc) as u8)),
                    chain_config,
                    cache,
                    middleware_map_req: Arc::new(QueryMiddlewareMapReq::default_list()),
                    middleware_map_resp: Arc::new(QueryMiddlewareMapResp::default_list()),
                    middleware_run: Arc::new(QueryMiddlewareRun::default_list()),
                    balances_pagination_limit: DEFAULT_BALANCES_PAGINATION_LIMIT,
                    wait_blocks_poll_sleep_duration: DEFAULT_WAIT_BLOCKS_POLL_SLEEP_DURATION,
                    _grpc_channel,
                    _rpc_client,
                    connection,
                };

                if _self.connection.preferred_mode.is_none() {
                    _self.set_connection_mode(None).await?;
                }


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

#[derive(Clone)]
pub struct Connection {
    // Todo - expand for gRPC, get rid of feature-gating
    pub rpc: Arc<dyn RpcTransport>,
    pub preferred_mode: Option<ConnectionMode>,
}

cfg_if::cfg_if! {
    // WASI
    if #[cfg(all(target_arch = "wasm32", not(target_os = "unknown")))] {
        impl Default for Connection {
            fn default() -> Self {
                Self {
                    rpc: Arc::new(crate::network::rpc::WasiRpcTransport{}),
                    preferred_mode: Some(ConnectionMode::Rpc),
                }
            }
        }
    } else {
        impl Default for Connection {
            fn default() -> Self {
                Self {
                    rpc: Arc::new(reqwest::Client::new()),
                    preferred_mode: None,
                }
            }
        }
    }
}

// currently only used via automatic fallback in very specific cases
// TODO: make this more general
#[derive(Clone, Copy, Debug)]
pub enum ConnectionMode {
    Grpc,
    Rpc,
}

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", not(target_os = "unknown")))] {
        impl ConnectionMode {
            pub fn modes_to_try() -> Vec<Self> {
                // WASI only supports RPC for now, don't even try anything else
                vec![Self::Rpc]
            }
        }
    } else {
        impl ConnectionMode {
            pub fn modes_to_try() -> Vec<Self> {
                vec![Self::Grpc, Self::Rpc]
            }
        }
    }
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

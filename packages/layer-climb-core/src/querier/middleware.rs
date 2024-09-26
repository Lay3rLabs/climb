pub mod logger;
pub mod retry;

use crate::prelude::*;
use logger::{QueryLoggerMiddlewareMapReq, QueryLoggerMiddlewareMapResp};
use retry::QueryRetryMiddleware;

pub enum QueryMiddlewareMapReq {
    Logger(QueryLoggerMiddlewareMapReq),
}

impl QueryMiddlewareMapReq {
    pub async fn map_req<REQ: QueryRequest>(&self, req: REQ) -> Result<REQ> {
        match self {
            Self::Logger(m) => m.map_req(req).await,
        }
    }
    pub fn default_list() -> Vec<Self> {
        vec![
            //Self::Logger(QueryLoggerMiddlewareMapReq::default()),
        ]
    }
}

pub enum QueryMiddlewareMapResp {
    Logger(QueryLoggerMiddlewareMapResp),
}

impl QueryMiddlewareMapResp {
    pub async fn map_resp<RESP: std::fmt::Debug + Send>(&self, resp: RESP) -> Result<RESP> {
        match self {
            Self::Logger(m) => m.map_resp(resp).await,
        }
    }
    pub fn default_list() -> Vec<Self> {
        vec![
            //Self::Logger(QueryLoggerMiddlewareMapResp::default()),
        ]
    }
}

pub enum QueryMiddlewareRun {
    Retry(QueryRetryMiddleware),
}

impl QueryMiddlewareRun {
    pub async fn run<REQ: QueryRequest>(
        &self,
        req: REQ,
        client: QueryClient,
    ) -> Result<REQ::QueryResponse> {
        match self {
            Self::Retry(m) => m.run(req, client).await,
        }
    }
    pub fn default_list() -> Vec<Self> {
        vec![Self::Retry(QueryRetryMiddleware::default())]
    }
}

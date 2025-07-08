use std::sync::Arc;

use crate::prelude::*;

#[derive(Clone)]
pub struct QueryLoggerMiddlewareMapReq {
    pub logger_fn: Arc<dyn Fn(String) + Send + Sync>,
}
impl QueryLoggerMiddlewareMapReq {
    pub fn new<F>(logger_fn: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        Self {
            logger_fn: Arc::new(logger_fn),
        }
    }
}
impl Default for QueryLoggerMiddlewareMapReq {
    fn default() -> Self {
        Self::new(|msg| eprintln!("{msg}"))
    }
}

impl QueryLoggerMiddlewareMapReq {
    pub async fn map_req<REQ: QueryRequest>(&self, req: REQ) -> Result<REQ> {
        (self.logger_fn)(format!("{req:?}"));
        Ok(req)
    }
}

pub struct QueryLoggerMiddlewareMapResp {
    pub logger_fn: Arc<dyn Fn(String) + Send + Sync>,
}
impl QueryLoggerMiddlewareMapResp {
    pub fn new<F>(logger_fn: F) -> Self
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        Self {
            logger_fn: Arc::new(logger_fn),
        }
    }
}
impl Default for QueryLoggerMiddlewareMapResp {
    fn default() -> Self {
        Self::new(|msg| eprintln!("{msg}"))
    }
}

impl QueryLoggerMiddlewareMapResp {
    pub async fn map_resp<RESP: std::fmt::Debug + Send>(&self, resp: RESP) -> Result<RESP> {
        (self.logger_fn)(format!("{resp:?}"));
        Ok(resp)
    }
}

use crate::{prelude::*, querier::tx::AnyTxResponse};

use std::sync::Arc;

#[derive(Clone)]
pub struct SigningLoggerMiddlewareMapBody {
    pub logger_fn: Arc<dyn Fn(&layer_climb_proto::tx::TxBody) + Send + Sync>,
}
impl SigningLoggerMiddlewareMapBody {
    pub fn new<F>(logger_fn: F) -> Self
    where
        F: Fn(&layer_climb_proto::tx::TxBody) + Send + Sync + 'static,
    {
        Self {
            logger_fn: Arc::new(logger_fn),
        }
    }
}
impl Default for SigningLoggerMiddlewareMapBody {
    fn default() -> Self {
        Self::new(|body| eprintln!("{:?}", body))
    }
}

impl SigningLoggerMiddlewareMapBody {
    pub async fn map_body(
        &self,
        body: layer_climb_proto::tx::TxBody,
    ) -> Result<layer_climb_proto::tx::TxBody> {
        (self.logger_fn)(&body);
        Ok(body)
    }
}

pub struct SigningLoggerMiddlewareMapResp {
    pub logger_fn: Arc<dyn Fn(&AnyTxResponse) + Send + Sync>,
}
impl SigningLoggerMiddlewareMapResp {
    pub fn new<F>(logger_fn: F) -> Self
    where
        F: Fn(&AnyTxResponse) + Send + Sync + 'static,
    {
        Self {
            logger_fn: Arc::new(logger_fn),
        }
    }
}
impl Default for SigningLoggerMiddlewareMapResp {
    fn default() -> Self {
        Self::new(|resp| eprintln!("{:?}", resp))
    }
}

impl SigningLoggerMiddlewareMapResp {
    pub async fn map_resp(&self, resp: AnyTxResponse) -> Result<AnyTxResponse> {
        (self.logger_fn)(&resp);
        Ok(resp)
    }
}

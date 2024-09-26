pub mod logger;

use crate::prelude::*;
use logger::{SigningLoggerMiddlewareMapBody, SigningLoggerMiddlewareMapResp};

pub enum SigningMiddlewareMapBody {
    Logger(SigningLoggerMiddlewareMapBody),
}

impl SigningMiddlewareMapBody {
    pub async fn map_body(
        &self,
        req: layer_climb_proto::tx::TxBody,
    ) -> Result<layer_climb_proto::tx::TxBody> {
        match self {
            Self::Logger(m) => m.map_body(req).await,
        }
    }
    pub fn default_list() -> Vec<Self> {
        vec![
            //Self::Logger(SigningLoggerMiddlewareMapBody::default()),
        ]
    }
}

pub enum SigningMiddlewareMapResp {
    Logger(SigningLoggerMiddlewareMapResp),
}

impl SigningMiddlewareMapResp {
    pub async fn map_resp(
        &self,
        resp: layer_climb_proto::abci::TxResponse,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        match self {
            Self::Logger(m) => m.map_resp(resp).await,
        }
    }
    pub fn default_list() -> Vec<Self> {
        vec![
            //Self::Logger(SigningLoggerMiddlewareMapResp::default()),
        ]
    }
}

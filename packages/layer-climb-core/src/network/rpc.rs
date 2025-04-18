use std::sync::Arc;

use crate::prelude::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tendermint_rpc::Response;

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        #[async_trait(?Send)]
        pub trait RpcTransport: Send + Sync {
            async fn post_json_bytes(&self, url: &str, body: Vec<u8>) -> anyhow::Result<String>;
        }

        cfg_if::cfg_if! {
            if #[cfg(target_os = "unknown")] {
                #[async_trait(?Send)]
                impl RpcTransport for reqwest::Client {
                    async fn post_json_bytes(&self, url: &str, body: Vec<u8>) -> anyhow::Result<String> {
                        self.post(url)
                            .header("Content-Type", "application/json")
                            .body(body)
                            .send()
                            .await
                            .map_err(|e| anyhow!("{}", e))?
                            .text()
                            .await
                            .map_err(|e| anyhow!("{}", e))
                    }
                }
            } else {
                use wstd::{
                    http::{Client, IntoBody, Request, StatusCode},
                    io::AsyncRead,
                };

                pub struct WasiRpcTransport {}

                // prior art, cloudflare does this trick too: https://github.com/cloudflare/workers-rs/blob/38af58acc4e54b29c73336c1720188f3c3e86cc4/worker/src/send.rs#L32
                unsafe impl Sync for WasiRpcTransport {}
                unsafe impl Send for WasiRpcTransport {}

                #[async_trait(?Send)]
                impl RpcTransport for WasiRpcTransport {
                    async fn post_json_bytes(&self, url: &str, body: Vec<u8>) -> anyhow::Result<String> {
                        let request = Request::post(url).header("content-type", "application/json").body(body.into_body())?;
                        let mut res = Client::new().send(request).await?;

                        match res.status() {
                            StatusCode::OK => {
                                let body = res.body_mut();
                                let mut body_buf = Vec::new();
                                body.read_to_end(&mut body_buf).await?;
                                String::from_utf8(body_buf).map_err(|err| anyhow::anyhow!(err))
                            },
                            status => Err(anyhow!("unexpected status code: {status}")),
                        }
                    }
                }
            }
        }
    } else {
        #[async_trait]
        pub trait RpcTransport: Send + Sync {
            async fn post_json_bytes(&self, url: &str, body: Vec<u8>) -> anyhow::Result<String>;
        }

        #[async_trait]
        impl RpcTransport for reqwest::Client {
            async fn post_json_bytes(&self, url: &str, body: Vec<u8>) -> anyhow::Result<String> {
                self.post(url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await
                    .map_err(|e| anyhow!("{}", e))?
                    .text()
                    .await
                    .map_err(|e| anyhow!("{}", e))
            }
        }
    }
}

#[derive(Clone)]
pub struct RpcClient {
    http_client: Arc<dyn RpcTransport>,
    url: String,
}

impl RpcClient {
    pub fn new(url: String, http_client: Arc<dyn RpcTransport>) -> Self {
        Self { url, http_client }
    }

    pub async fn commit(&self, height: u64) -> Result<tendermint_rpc::endpoint::commit::Response> {
        let height = tendermint::block::Height::try_from(height)?;
        self.send(tendermint_rpc::endpoint::commit::Request::new(height))
            .await
    }

    pub async fn broadcast_tx(
        &self,
        tx: Vec<u8>,
        mode: layer_climb_proto::tx::BroadcastMode,
    ) -> Result<TxResponse> {
        match mode {
            layer_climb_proto::tx::BroadcastMode::Sync
            | layer_climb_proto::tx::BroadcastMode::Block => self
                .send(tendermint_rpc::endpoint::broadcast::tx_sync::Request::new(
                    tx,
                ))
                .await
                .map(|resp| resp.into()),
            layer_climb_proto::tx::BroadcastMode::Async => self
                .send(tendermint_rpc::endpoint::broadcast::tx_async::Request::new(
                    tx,
                ))
                .await
                .map(|resp| resp.into()),
            layer_climb_proto::tx::BroadcastMode::Unspecified => {
                Err(anyhow!("broadcast mode unspecified"))
            }
        }
    }

    pub async fn block_results(
        &self,
        height: u64,
    ) -> Result<tendermint_rpc::endpoint::block_results::Response> {
        let height = tendermint::block::Height::try_from(height)?;
        self.send(tendermint_rpc::endpoint::block_results::Request::new(
            height,
        ))
        .await
    }

    pub async fn block(
        &self,
        height: Option<u64>,
    ) -> Result<tendermint_rpc::endpoint::block::v0_38::DialectResponse> {
        self.send(tendermint_rpc::endpoint::block::Request {
            height: height.map(|h| h.try_into()).transpose()?,
        })
        .await
    }

    pub async fn health(&self) -> Result<tendermint_rpc::endpoint::health::Response> {
        self.send(tendermint_rpc::endpoint::health::Request).await
    }

    pub async fn abci_query(
        &self,
        path: String,
        data: Vec<u8>,
        height: Option<u64>,
        prove: bool,
    ) -> Result<tendermint_rpc::endpoint::abci_query::AbciQuery> {
        let height = match height {
            Some(height) => Some(tendermint::block::Height::try_from(height)?),
            None => {
                // according to the rpc docs, 0 is latest... not sure what native None means
                Some(tendermint::block::Height::try_from(0u64)?)
            }
        };
        let resp = self
            .send(tendermint_rpc::endpoint::abci_query::Request {
                path: Some(path),
                data,
                height,
                prove,
            })
            .await?
            .response;

        if resp.code.is_err() {
            bail!("abci query failed: {}", resp.log);
        }

        Ok(resp)
    }

    pub async fn abci_protobuf_query<REQ, RESP>(
        &self,
        path: impl ToString,
        req: REQ,
        height: Option<u64>,
    ) -> Result<RESP>
    where
        REQ: layer_climb_proto::Name,
        RESP: layer_climb_proto::Name + Default,
    {
        let resp = self
            .abci_query(path.to_string(), req.encode_to_vec(), height, false)
            .await?;

        RESP::decode(resp.value.as_slice()).map_err(|err| anyhow::anyhow!(err))
    }

    async fn send<T: tendermint_rpc::Request>(&self, req: T) -> Result<T::Response> {
        let res = self
            .http_client
            .post_json_bytes(&self.url, req.into_json().into_bytes())
            .await?;

        T::Response::from_string(res).map_err(|err| anyhow::anyhow!(err))
    }
}

/// Response from any kind of transaction broadcast request.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TxResponse {
    /// Code space
    pub codespace: String,

    /// Code
    pub code: tendermint::abci::Code,

    /// Data
    pub data: Vec<u8>,

    /// Log
    pub log: String,

    /// Transaction hash
    pub hash: tendermint::Hash,
}

impl From<tendermint_rpc::endpoint::broadcast::tx_sync::Response> for TxResponse {
    fn from(resp: tendermint_rpc::endpoint::broadcast::tx_sync::Response) -> Self {
        Self {
            codespace: resp.codespace,
            code: resp.code,
            data: resp.data.into(),
            log: resp.log,
            hash: resp.hash,
        }
    }
}

impl From<tendermint_rpc::endpoint::broadcast::tx_async::Response> for TxResponse {
    fn from(resp: tendermint_rpc::endpoint::broadcast::tx_async::Response) -> Self {
        Self {
            codespace: resp.codespace,
            code: resp.code,
            data: resp.data.into(),
            log: resp.log,
            hash: resp.hash,
        }
    }
}

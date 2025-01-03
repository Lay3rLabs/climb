use crate::prelude::*;
use serde::{Deserialize, Serialize};
use tendermint_rpc::Response;

#[derive(Clone, Debug)]
pub struct RpcClient {
    http_client: reqwest::Client,
    url: String,
}

impl RpcClient {
    pub fn new(url: String, http_client: reqwest::Client) -> Self {
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
    ) -> Result<tendermint_rpc::endpoint::block::Response> {
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
            .post(self.url.clone())
            .header("Content-Type", "application/json")
            .body(req.into_json().into_bytes())
            .send()
            .await?
            .text()
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

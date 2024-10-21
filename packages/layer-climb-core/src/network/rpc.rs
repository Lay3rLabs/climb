use crate::prelude::*;
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

    pub async fn health(&self) -> Result<tendermint_rpc::endpoint::health::Response> {
        self.send(tendermint_rpc::endpoint::health::Request).await
    }

    pub async fn abci_query(
        &self,
        path: String,
        data: Vec<u8>,
        height: u64,
        prove: bool,
    ) -> Result<tendermint_rpc::endpoint::abci_query::Response> {
        self.send(tendermint_rpc::endpoint::abci_query::Request {
            path: Some(path),
            data,
            height: Some(tendermint::block::Height::try_from(height)?),
            prove,
        })
        .await
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

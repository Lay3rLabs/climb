// these do not go through request middleware since they are transaction-related
// but they are part of QueryClient, not SigningClient

use std::time::Duration;

use crate::prelude::*;

impl QueryClient {
    pub async fn simulate_tx(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<layer_climb_proto::tx::SimulateResponse> {
        #[allow(deprecated)]
        let req = layer_climb_proto::tx::SimulateRequest { tx: None, tx_bytes };

        match self.get_connection_mode() {
            ConnectionMode::Grpc => {
                let mut query_client = layer_climb_proto::tx::service_client::ServiceClient::new(
                    self.clone_grpc_channel()?,
                );

                query_client
                    .simulate(req)
                    .await
                    .map(|res| res.into_inner())
                    .map_err(|e| anyhow!("couldn't simulate tx: {e:?}"))
            }
            ConnectionMode::Rpc => self
                .rpc_client()?
                .abci_protobuf_query("/cosmos.tx.v1beta1.Service/Simulate", req, None)
                .await
                .map_err(|e| anyhow!("couldn't simulate tx: {e:?}")),
        }
    }

    pub async fn broadcast_tx_bytes(
        &self,
        tx_bytes: Vec<u8>,
        mode: layer_climb_proto::tx::BroadcastMode,
    ) -> Result<AnyTxResponse> {
        match self.get_connection_mode() {
            ConnectionMode::Grpc => {
                let req = layer_climb_proto::tx::BroadcastTxRequest {
                    tx_bytes,
                    mode: mode.into(),
                };

                let mut query_client = layer_climb_proto::tx::service_client::ServiceClient::new(
                    self.clone_grpc_channel()?,
                );

                query_client
                    .broadcast_tx(req)
                    .await
                    .map(|res| res.into_inner().tx_response)?
                    .context("couldn't broadcast tx")
                    .map(AnyTxResponse::Abci)
            }
            ConnectionMode::Rpc => self
                .rpc_client()?
                .broadcast_tx(tx_bytes, mode)
                .await
                .context("couldn't broadcast tx")
                .map(AnyTxResponse::Rpc),
        }
    }

    #[tracing::instrument]
    pub async fn poll_until_tx_ready(
        &self,
        tx_hash: String,
        sleep_duration: Duration,
        timeout_duration: Duration,
    ) -> Result<PollTxResponse> {
        let mut total_duration = Duration::default();

        let mut grpc_query_client = match self.get_connection_mode() {
            ConnectionMode::Grpc => {
                Some(layer_climb_proto::tx::service_client::ServiceClient::new(
                    self.clone_grpc_channel()?,
                ))
            }
            ConnectionMode::Rpc => None,
        };

        loop {
            let req = layer_climb_proto::tx::GetTxRequest {
                hash: tx_hash.clone(),
            };

            let response = match self.get_connection_mode() {
                ConnectionMode::Grpc => {
                    let res = grpc_query_client
                        .as_mut()
                        .unwrap()
                        .get_tx(req)
                        .await
                        .map(|res| {
                            let inner = res.into_inner();
                            (inner.tx, inner.tx_response)
                        });

                    match res {
                        Ok(res) => Ok(Some(res)),
                        Err(e) => {
                            if e.code() == tonic::Code::Ok || e.code() == tonic::Code::NotFound {
                                Ok(None)
                            } else {
                                tracing::debug!(
                                    "failed grpc GetTxRequest [code: {}]. Full error: {:?}",
                                    e.code(),
                                    e
                                );
                                Err(e.into())
                            }
                        }
                    }
                }
                ConnectionMode::Rpc => {
                    let res = self
                        .rpc_client()?
                        .abci_protobuf_query::<_, layer_climb_proto::tx::GetTxResponse>(
                            "/cosmos.tx.v1beta1.Service/GetTx",
                            req,
                            None,
                        )
                        .await
                        .map(|res| (res.tx, res.tx_response));

                    match res {
                        Ok(res) => Ok(Some(res)),
                        Err(e) => {
                            // eww :/
                            if e.to_string().to_lowercase().contains("notfound") {
                                Ok(None)
                            } else {
                                tracing::debug!("failed rpc GetTxRequest. Full error: {:?}", e);
                                Err(e)
                            }
                        }
                    }
                }
            };

            match response {
                Ok(Some((tx, Some(tx_response)))) => {
                    return Ok(PollTxResponse { tx, tx_response });
                }
                Err(e) => {
                    return Err(e);
                }
                _ => {}
            }

            futures_timer::Delay::new(sleep_duration).await;
            total_duration += sleep_duration;
            if total_duration >= timeout_duration {
                return Err(anyhow!("timeout"));
            }
        }
    }
}

pub struct PollTxResponse {
    pub tx: Option<layer_climb_proto::tx::Tx>,
    pub tx_response: layer_climb_proto::abci::TxResponse,
}

#[derive(Debug)]
pub enum AnyTxResponse {
    Abci(layer_climb_proto::abci::TxResponse),
    Rpc(crate::network::rpc::TxResponse),
}

impl AnyTxResponse {
    pub fn code(&self) -> u32 {
        match self {
            AnyTxResponse::Abci(res) => res.code,
            AnyTxResponse::Rpc(res) => match res.code {
                tendermint::abci::Code::Ok => 0,
                tendermint::abci::Code::Err(non_zero) => non_zero.into(),
            },
        }
    }

    pub fn codespace(&self) -> &str {
        match self {
            AnyTxResponse::Abci(res) => &res.codespace,
            AnyTxResponse::Rpc(res) => &res.codespace,
        }
    }

    pub fn raw_log(&self) -> &str {
        match self {
            AnyTxResponse::Abci(res) => &res.raw_log,
            AnyTxResponse::Rpc(res) => &res.log,
        }
    }

    pub fn tx_hash(&self) -> String {
        match self {
            AnyTxResponse::Abci(res) => res.txhash.clone(),
            AnyTxResponse::Rpc(res) => res.hash.to_string(),
        }
    }
}

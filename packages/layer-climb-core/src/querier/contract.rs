use crate::{contract_helpers::contract_msg_to_vec, prelude::*};
use serde::{de::DeserializeOwned, Serialize};
use tracing::instrument;

impl QueryClient {
    #[instrument]
    pub async fn contract_smart<
        D: DeserializeOwned + Send + std::fmt::Debug + Sync,
        S: Serialize + std::fmt::Debug,
    >(
        &self,
        address: &Address,
        msg: &S,
    ) -> Result<D> {
        self.run_with_middleware(ContractSmartReq {
            address: address.clone(),
            msg: contract_msg_to_vec(&msg)?,
            _phantom: std::marker::PhantomData,
        })
        .await
    }
    #[instrument]
    pub async fn contract_smart_raw<S: Serialize + std::fmt::Debug>(
        &self,
        address: &Address,
        msg: &S,
    ) -> Result<Vec<u8>> {
        self.run_with_middleware(ContractSmartRawReq {
            address: address.clone(),
            msg: contract_msg_to_vec(&msg)?,
        })
        .await
    }

    #[instrument]
    pub async fn contract_code_info(
        &self,
        code_id: u64,
    ) -> Result<layer_climb_proto::wasm::CodeInfoResponse> {
        self.run_with_middleware(ContractCodeInfoReq { code_id })
            .await
    }

    #[instrument]
    pub async fn contract_info(
        &self,
        address: &Address,
    ) -> Result<layer_climb_proto::wasm::QueryContractInfoResponse> {
        self.run_with_middleware(ContractInfoReq {
            address: address.clone(),
        })
        .await
    }
}

#[derive(Debug)]
struct ContractSmartReq<D> {
    pub address: Address,
    pub msg: Vec<u8>,
    _phantom: std::marker::PhantomData<D>,
}

impl<D> Clone for ContractSmartReq<D> {
    fn clone(&self) -> Self {
        Self {
            address: self.address.clone(),
            msg: self.msg.clone(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<D: DeserializeOwned + Send + std::fmt::Debug + Sync> QueryRequest for ContractSmartReq<D> {
    type QueryResponse = D;

    async fn request(&self, client: QueryClient) -> Result<D> {
        let res = ContractSmartRawReq {
            address: self.address.clone(),
            msg: self.msg.clone(),
        }
        .request(client)
        .await?;

        let res = cosmwasm_std::from_json(res)
            .map_err(|e| anyhow::anyhow!("couldn't deserialize response {}", e))?;

        Ok(res)
    }
}

#[derive(Clone, Debug)]
struct ContractSmartRawReq {
    pub address: Address,
    pub msg: Vec<u8>,
}

impl QueryRequest for ContractSmartRawReq {
    type QueryResponse = Vec<u8>;

    async fn request(&self, client: QueryClient) -> Result<Vec<u8>> {
        let req = layer_climb_proto::wasm::QuerySmartContractStateRequest {
            address: self.address.to_string(),
            query_data: self.msg.clone(),
        };

        let res = match client.get_connection_mode() {
            ConnectionMode::Grpc => {
                let mut query_client = layer_climb_proto::wasm::query_client::QueryClient::new(
                    client.clone_grpc_channel()?,
                );

                query_client
                    .smart_contract_state(req)
                    .await
                    .map(|res| res.into_inner())?
            }
            ConnectionMode::Rpc => {
                client
                    .rpc_client()?
                    .abci_protobuf_query("/cosmwasm.wasm.v1.Query/SmartContractState", req, None)
                    .await?
            }
        };

        Ok(res.data)
    }
}

#[derive(Clone, Debug)]
struct ContractCodeInfoReq {
    pub code_id: u64,
}

impl QueryRequest for ContractCodeInfoReq {
    type QueryResponse = layer_climb_proto::wasm::CodeInfoResponse;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::wasm::CodeInfoResponse> {
        let req = layer_climb_proto::wasm::QueryCodeRequest {
            code_id: self.code_id,
        };

        let res = match client.get_connection_mode() {
            ConnectionMode::Grpc => {
                let mut query_client = layer_climb_proto::wasm::query_client::QueryClient::new(
                    client.clone_grpc_channel()?,
                );

                query_client.code(req).await.map(|res| res.into_inner())?
            }
            ConnectionMode::Rpc => {
                client
                    .rpc_client()?
                    .abci_protobuf_query("/cosmwasm.wasm.v1.Query/Code", req, None)
                    .await?
            }
        };

        res.code_info.context("no code info found")
    }
}

#[derive(Clone, Debug)]
pub struct ContractInfoReq {
    pub address: Address,
}

impl QueryRequest for ContractInfoReq {
    type QueryResponse = layer_climb_proto::wasm::QueryContractInfoResponse;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::wasm::QueryContractInfoResponse> {
        let req = layer_climb_proto::wasm::QueryContractInfoRequest {
            address: self.address.to_string(),
        };

        match client.get_connection_mode() {
            ConnectionMode::Grpc => {
                let mut query_client = layer_climb_proto::wasm::query_client::QueryClient::new(
                    client.clone_grpc_channel()?,
                );

                let resp = query_client
                    .contract_info(req)
                    .await
                    .map(|res| res.into_inner())?;

                Ok(resp)
            }
            ConnectionMode::Rpc => {
                let resp = client
                    .rpc_client()?
                    .abci_protobuf_query::<_, layer_climb_proto::wasm::QueryContractInfoResponse>(
                        "/cosmwasm.wasm.v1.Query/ContractInfo",
                        req,
                        None,
                    )
                    .await?;

                Ok(resp)
            }
        }
    }
}

use tracing::instrument;

use crate::prelude::*;

impl QueryClient {
    #[instrument]
    pub async fn balance(&self, addr: Address, denom: Option<String>) -> Result<Option<u128>> {
        self.run_with_middleware(BalanceReq { addr, denom }).await
    }

    #[instrument]
    pub async fn all_balances(
        &self,
        addr: Address,
        limit_per_page: Option<u64>,
    ) -> Result<Vec<layer_climb_proto::Coin>> {
        self.run_with_middleware(AllBalancesReq {
            addr,
            limit_per_page,
        })
        .await
    }

    #[instrument]
    pub async fn base_account(
        &self,
        addr: &Address,
    ) -> Result<layer_climb_proto::auth::BaseAccount> {
        self.run_with_middleware(BaseAccountReq { addr: addr.clone() })
            .await
    }

    #[instrument]
    pub async fn staking_params(&self) -> Result<layer_climb_proto::staking::Params> {
        self.run_with_middleware(StakingParamsReq {}).await
    }

    #[instrument]
    pub async fn block(&self, height: Option<u64>) -> Result<BlockResp> {
        self.run_with_middleware(BlockReq { height }).await
    }

    #[instrument]
    pub async fn block_header(&self, height: Option<u64>) -> Result<BlockHeaderResp> {
        self.run_with_middleware(BlockHeaderReq { height }).await
    }

    #[instrument]
    pub async fn block_height(&self) -> Result<u64> {
        self.run_with_middleware(BlockHeightReq {}).await
    }
}

#[derive(Clone, Debug)]
pub struct BalanceReq {
    pub addr: Address,
    pub denom: Option<String>,
}

impl QueryRequest for BalanceReq {
    type QueryResponse = Option<u128>;

    async fn request(&self, client: QueryClient) -> Result<Self::QueryResponse> {
        let mut query_client =
            layer_climb_proto::bank::query_client::QueryClient::new(client.grpc_channel.clone());

        let denom = self
            .denom
            .clone()
            .unwrap_or(client.chain_config.gas_denom.clone());

        let coin = query_client
            .balance(layer_climb_proto::bank::QueryBalanceRequest {
                address: self.addr.to_string(),
                denom,
            })
            .await
            .map(|res| res.into_inner().balance)?;

        match coin {
            None => Ok(None),
            Some(coin) => {
                let amount = coin
                    .amount
                    .parse::<u128>()
                    .context("couldn't parse amount")?;
                Ok(Some(amount))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct AllBalancesReq {
    pub addr: Address,
    pub limit_per_page: Option<u64>,
}

impl QueryRequest for AllBalancesReq {
    type QueryResponse = Vec<layer_climb_proto::Coin>;

    async fn request(&self, client: QueryClient) -> Result<Self::QueryResponse> {
        let mut query_client =
            layer_climb_proto::bank::query_client::QueryClient::new(client.grpc_channel.clone());

        let mut coins = Vec::new();

        let mut pagination = None;

        let limit = self
            .limit_per_page
            .unwrap_or(client.balances_pagination_limit);

        loop {
            let resp = query_client
                .all_balances(layer_climb_proto::bank::QueryAllBalancesRequest {
                    address: self.addr.to_string(),
                    pagination,
                    resolve_denom: true,
                })
                .await
                .map(|res| res.into_inner())?;

            coins.extend(resp.balances);

            match &resp.pagination {
                None => break,
                Some(pagination_response) => {
                    if pagination_response.next_key.is_empty() {
                        break;
                    }
                }
            }

            pagination = resp
                .pagination
                .map(|p| layer_climb_proto::query::PageRequest {
                    key: p.next_key,
                    offset: 0,
                    limit,
                    count_total: false,
                    reverse: false,
                });
        }

        Ok(coins)
    }
}

#[derive(Clone, Debug)]
pub struct BaseAccountReq {
    pub addr: Address,
}

impl QueryRequest for BaseAccountReq {
    type QueryResponse = layer_climb_proto::auth::BaseAccount;

    async fn request(&self, client: QueryClient) -> Result<Self::QueryResponse> {
        let mut query_client =
            layer_climb_proto::auth::query_client::QueryClient::new(client.grpc_channel.clone());

        let account = query_client
            .account(layer_climb_proto::auth::QueryAccountRequest {
                address: self.addr.to_string(),
            })
            .await
            .map(|res| res.into_inner().account)?
            .ok_or_else(|| anyhow!("account {} not found", self.addr))?;

        let account = layer_climb_proto::auth::BaseAccount::decode(account.value.as_slice())
            .context("couldn't decode account")?;

        Ok(account)
    }
}

#[derive(Clone, Debug)]
pub struct StakingParamsReq {}

impl QueryRequest for StakingParamsReq {
    type QueryResponse = layer_climb_proto::staking::Params;

    async fn request(&self, client: QueryClient) -> Result<layer_climb_proto::staking::Params> {
        let mut query_client =
            layer_climb_proto::staking::query_client::QueryClient::new(client.grpc_channel.clone());

        let resp = query_client
            .params(layer_climb_proto::staking::QueryParamsRequest {})
            .await
            .map(|res| res.into_inner())
            .context("couldn't get staking params")?;

        resp.params.ok_or(anyhow!("no staking params found"))
    }
}

#[derive(Clone, Debug)]
pub struct BlockReq {
    pub height: Option<u64>,
}

#[derive(Debug)]
pub enum BlockResp {
    Sdk(layer_climb_proto::block::SdkBlock),
    Old(layer_climb_proto::block::TendermintBlock),
}

impl QueryRequest for BlockReq {
    type QueryResponse = BlockResp;

    async fn request(&self, client: QueryClient) -> Result<Self::QueryResponse> {
        let mut query_client = layer_climb_proto::tendermint::service_client::ServiceClient::new(
            client.grpc_channel.clone(),
        );
        let height = self.height;

        match height {
            Some(height) => query_client
                .get_block_by_height(layer_climb_proto::tendermint::GetBlockByHeightRequest {
                    height: height.try_into()?,
                })
                .await
                .map_err(|err| err.into())
                .and_then(|res| {
                    let res = res.into_inner();
                    match res.sdk_block {
                        Some(block) => Ok(BlockResp::Sdk(block)),
                        None => res
                            .block
                            .map(BlockResp::Old)
                            .ok_or(anyhow!("no block found")),
                    }
                }),
            None => query_client
                .get_latest_block(layer_climb_proto::tendermint::GetLatestBlockRequest {})
                .await
                .map_err(|err| err.into())
                .and_then(|res| {
                    let res = res.into_inner();
                    match res.sdk_block {
                        Some(block) => Ok(BlockResp::Sdk(block)),
                        None => res
                            .block
                            .map(BlockResp::Old)
                            .ok_or(anyhow!("no block found")),
                    }
                }),
        }
        .with_context(move || match height {
            Some(height) => format!("no block found at height {}", height),
            None => "no latest block found".to_string(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct BlockHeaderReq {
    pub height: Option<u64>,
}

#[derive(Debug)]
pub enum BlockHeaderResp {
    Sdk(layer_climb_proto::block::SdkHeader),
    Old(layer_climb_proto::block::TendermintHeader),
}

impl BlockHeaderResp {
    pub fn height(&self) -> Result<u64> {
        Ok(match self {
            BlockHeaderResp::Sdk(header) => header.height.try_into()?,
            BlockHeaderResp::Old(header) => header.height.try_into()?,
        })
    }

    pub fn time(&self) -> Option<layer_climb_proto::Timestamp> {
        match self {
            BlockHeaderResp::Sdk(header) => header.time,
            BlockHeaderResp::Old(header) => header.time.map(|time| layer_climb_proto::Timestamp {
                seconds: time.seconds,
                nanos: time.nanos,
            }),
        }
    }

    pub fn app_hash(&self) -> Vec<u8> {
        match self {
            BlockHeaderResp::Sdk(header) => header.app_hash.clone(),
            BlockHeaderResp::Old(header) => header.app_hash.clone(),
        }
    }

    pub fn next_validators_hash(&self) -> Vec<u8> {
        match self {
            BlockHeaderResp::Sdk(header) => header.next_validators_hash.clone(),
            BlockHeaderResp::Old(header) => header.next_validators_hash.clone(),
        }
    }
}

impl QueryRequest for BlockHeaderReq {
    type QueryResponse = BlockHeaderResp;

    async fn request(&self, client: QueryClient) -> Result<Self::QueryResponse> {
        let block = BlockReq {
            height: self.height,
        }
        .request(client)
        .await?;

        match block {
            BlockResp::Sdk(block) => Ok(BlockHeaderResp::Sdk(
                block.header.context("no header found")?,
            )),
            BlockResp::Old(block) => Ok(BlockHeaderResp::Old(
                block.header.context("no header found")?,
            )),
        }
    }
}

#[derive(Clone, Debug)]
pub struct BlockHeightReq {}

impl QueryRequest for BlockHeightReq {
    type QueryResponse = u64;

    async fn request(&self, client: QueryClient) -> Result<u64> {
        let header = BlockHeaderReq { height: None }.request(client).await?;

        Ok(match header {
            BlockHeaderResp::Sdk(header) => header.height,
            BlockHeaderResp::Old(header) => header.height,
        }
        .try_into()?)
    }
}

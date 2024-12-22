// These were used to help debug and develop the client
// ideally they would be solely a backup (like the way abci_proof works)
// some old implementation code is kept in comments for reference

use tracing::instrument;

use super::basic::BlockHeightReq;
use crate::prelude::*;

impl QueryClient {
    #[instrument]
    pub async fn fetch_signed_header(
        &self,
        height: Option<u64>,
    ) -> Result<layer_climb_proto::tendermint::SignedHeader> {
        self.run_with_middleware(SignedHeaderReq { height }).await
    }
    #[instrument]
    pub async fn fetch_block_events(
        &self,
        block_height: u64,
    ) -> Result<Vec<tendermint::abci::Event>> {
        self.run_with_middleware(BlockEventsReq {
            height: block_height,
        })
        .await
    }
}

#[derive(Clone, Debug)]
struct SignedHeaderReq {
    pub height: Option<u64>,
}

impl QueryRequest for SignedHeaderReq {
    type QueryResponse = layer_climb_proto::tendermint::SignedHeader;

    async fn request(
        &self,
        client: QueryClient,
    ) -> Result<layer_climb_proto::tendermint::SignedHeader> {
        let height = match self.height {
            Some(height) => height,
            None => BlockHeightReq {}.request(client.clone()).await?,
        };

        // only exists on RPC?
        Ok(client
            .rpc_client()?
            .commit(height)
            .await?
            .signed_header
            .into())
    }
}

#[derive(Clone, Debug)]
struct BlockEventsReq {
    pub height: u64,
}

impl QueryRequest for BlockEventsReq {
    type QueryResponse = Vec<tendermint::abci::Event>;

    async fn request(&self, client: QueryClient) -> Result<Vec<tendermint::abci::Event>> {
        // Only exists on RPC?
        let mut response = client.rpc_client()?.block_results(self.height).await?;

        let mut events: Vec<tendermint::abci::Event> = vec![];

        if let Some(begin_block_events) = &mut response.begin_block_events {
            events.append(begin_block_events);
        }

        if let Some(txs_results) = &mut response.txs_results {
            for tx_result in txs_results {
                if tx_result.code != tendermint::abci::Code::Ok {
                    // Transaction failed, skip it
                    continue;
                }

                events.append(&mut tx_result.events);
            }
        }

        if let Some(end_block_events) = &mut response.end_block_events {
            events.append(end_block_events);
        }

        Ok(events)
    }
}

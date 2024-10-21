use std::time::Duration;

use crate::prelude::*;
use futures::Stream;
use tracing::instrument;

#[derive(Debug, Clone)]
pub struct BlockEvents {
    pub height: u64,
    pub events: Vec<tendermint::abci::Event>,
}

impl QueryClient {
    #[instrument]
    pub async fn stream_block_events(
        // take by value to avoid lifetime issues
        // typically this means the caller is cloning the QueryClient
        self,
        sleep_duration: Option<Duration>,
    ) -> Result<impl Stream<Item = Result<BlockEvents>>> {
        let start_height = self.block_height().await?;

        Ok(futures::stream::unfold(
            (self, start_height),
            move |(client, block_height)| async move {
                match client
                    .wait_until_block_height(block_height, sleep_duration)
                    .await
                {
                    Ok(_) => match client.fetch_block_events(block_height).await {
                        Err(err) => Some((Err(err), (client, block_height))),
                        Ok(events) => Some((
                            Ok(BlockEvents {
                                height: block_height,
                                events,
                            }),
                            (client, block_height + 1),
                        )),
                    },
                    Err(err) => Some((Err(err), (client, block_height))),
                }
            },
        ))
    }
}

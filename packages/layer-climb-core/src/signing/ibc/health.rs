use crate::{prelude::*, querier::abci::AbciProofKind};

impl SigningClient {
    // sanity check that the node has everything we need to do stuff
    // returns the tendermint version info
    pub async fn ibc_check_compat(&self) -> Result<layer_climb_proto::tendermint::VersionInfo> {
        let _ = self
            .querier
            .rpc_client
            .health()
            .await
            .context("couldn't get health over rpc")?;

        let node_info_resp = layer_climb_proto::tendermint::service_client::ServiceClient::new(
            self.querier.grpc_channel.clone(),
        )
        .get_node_info(layer_climb_proto::tendermint::GetNodeInfoRequest {})
        .await
        .map(|resp| resp.into_inner())
        .context("couldn't get status over grpc")?;

        let version = node_info_resp
            .application_version
            .context("missing application version")?;

        let height = self.querier.block_height().await?;

        self.querier
            .abci_proof(AbciProofKind::StakingParams, height)
            .await
            .context("couldn't get staking params proof")?;

        // let _ = self.http_client().rpc.tx_search(
        //     tendermint_rpc::query::Query::from(tendermint_rpc::query::EventType::NewBlock),
        //     false, 1, 1,
        //     tendermint_rpc::Order::Ascending
        // ).await.context("couldn't get tx search")?;

        Ok(version)
    }
}

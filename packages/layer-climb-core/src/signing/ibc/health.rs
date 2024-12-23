use crate::{prelude::*, querier::abci::AbciProofKind};

impl SigningClient {
    // sanity check that the node has everything we need to do stuff
    // returns the tendermint version info
    pub async fn ibc_check_compat(&self) -> Result<layer_climb_proto::tendermint::VersionInfo> {
        let _ = self
            .querier
            .rpc_client()?
            .health()
            .await
            .context("couldn't get health over rpc")?;

        let node_info_resp = self.querier.node_info().await?;

        let version = node_info_resp
            .application_version
            .context("missing application version")?;

        self.querier
            .abci_proof(AbciProofKind::StakingParams, None)
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

use layer_climb_proto::authz::Grant;

use crate::prelude::*;

impl SigningClient {
    pub async fn authz_grant_any(
        &self,
        granter: Option<Address>,
        grantee: Address,
        grant: Option<Grant>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(
                &self.authz_grant_any_msg(granter, grantee, grant)?,
            )?])
            .await?;

        Ok(resp)
    }

    pub async fn authz_grant_send(
        &self,
        granter: Option<Address>,
        grantee: Address,
        spend_limit: Vec<layer_climb_proto::Coin>,
        allow_list: Vec<Address>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&self.authz_grant_send_msg(
                granter,
                grantee,
                spend_limit,
                allow_list,
            )?)?])
            .await?;

        Ok(resp)
    }
}

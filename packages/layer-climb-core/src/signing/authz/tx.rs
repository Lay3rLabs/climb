use layer_climb_proto::authz::Grant;

use crate::prelude::*;

impl SigningClient {
    pub async fn authz_grant_any(
        &self,
        granter: Address,
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
        granter: Address,
        grantee: Address,
        spend_limit: Vec<layer_climb_proto::Coin>,
        allow_list: Vec<Address>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let grant = Grant {
            authorization: Some(proto_into_any(
                &layer_climb_proto::bank::SendAuthorization {
                    spend_limit,
                    allow_list: allow_list.into_iter().map(|a| a.to_string()).collect(),
                },
            )?),
            expiration: None,
        };

        self.authz_grant_any(granter, grantee, Some(grant), tx_builder)
            .await
    }
}

use anyhow::Result;
use layer_climb_proto::authz::Grant;

use crate::prelude::*;

impl SigningClient {
    pub fn authz_grant_any_msg(
        &self,
        granter: Option<Address>,
        grantee: Address,
        grant: Option<Grant>,
    ) -> Result<layer_climb_proto::authz::MsgGrant> {
        Ok(layer_climb_proto::authz::MsgGrant {
            granter: granter
                .map(|a| a.to_string())
                .unwrap_or_else(|| self.addr.to_string()),
            grantee: grantee.to_string(),
            grant,
        })
    }

    pub fn authz_grant_send_msg(
        &self,
        granter: Option<Address>,
        grantee: Address,
        spend_limit: Vec<layer_climb_proto::Coin>,
        allow_list: Vec<Address>,
    ) -> Result<layer_climb_proto::authz::MsgGrant> {
        let grant = Grant {
            authorization: Some(proto_into_any(
                &layer_climb_proto::bank::SendAuthorization {
                    spend_limit,
                    allow_list: allow_list.into_iter().map(|a| a.to_string()).collect(),
                },
            )?),
            expiration: None,
        };

        self.authz_grant_any_msg(granter, grantee, Some(grant))
    }
}

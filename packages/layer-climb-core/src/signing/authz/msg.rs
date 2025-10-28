use anyhow::Result;
use layer_climb_proto::authz::Grant;

use crate::prelude::*;

impl SigningClient {
    pub fn authz_grant_any_msg(
        &self,
        granter: Address,
        grantee: Address,
        grant: Option<Grant>,
    ) -> Result<layer_climb_proto::authz::MsgGrant> {
        Ok(layer_climb_proto::authz::MsgGrant {
            granter: granter.to_string(),
            grantee: grantee.to_string(),
            grant,
        })
    }
}

use crate::prelude::*;

impl SigningClient {
    pub fn transfer_msg<'a, D: Into<Option<&'a str>> + std::fmt::Debug>(
        &self,
        amount: u128,
        recipient: &Address,
        denom: D,
    ) -> Result<layer_climb_proto::bank::MsgSend> {
        let denom = denom.into().unwrap_or(&self.querier.chain_config.gas_denom);

        let amount = layer_climb_proto::Coin {
            amount: amount.to_string(),
            denom: denom.to_string(),
        };

        Ok(layer_climb_proto::bank::MsgSend {
            from_address: self.addr.to_string(),
            to_address: recipient.to_string(),
            amount: vec![amount],
        })
    }
}

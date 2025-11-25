use serde::Serialize;

use crate::{
    events::{
        EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V1, EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V2,
        EVENT_ATTR_STORE_CODE_ID, EVENT_TYPE_CONTRACT_INSTANTIATE, EVENT_TYPE_CONTRACT_STORE_CODE,
    },
    prelude::*,
};

impl SigningClient {
    // returns the code id
    pub async fn contract_upload_file(
        &self,
        wasm_byte_code: Vec<u8>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<(u64, layer_climb_proto::abci::TxResponse)> {
        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(
                &self.contract_upload_file_msg(wasm_byte_code)?,
            )?])
            .await?;

        let code_id: u64 = CosmosTxEvents::from(&resp)
            .attr_first(EVENT_TYPE_CONTRACT_STORE_CODE, EVENT_ATTR_STORE_CODE_ID)?
            .value()
            .parse()?;

        Ok((code_id, resp))
    }

    pub async fn contract_instantiate(
        &self,
        admin: impl Into<Option<Address>>,
        code_id: u64,
        label: impl ToString,
        msg: &impl Serialize,
        funds: Vec<layer_climb_proto::Coin>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<(Address, layer_climb_proto::abci::TxResponse)> {
        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(
                &self.contract_instantiate_msg(admin, code_id, label, funds, msg)?,
            )?])
            .await?;

        let events = CosmosTxEvents::from(&resp);

        let contract_address = events
            .attr_first(
                EVENT_TYPE_CONTRACT_INSTANTIATE,
                EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V1,
            )
            .or_else(|_| {
                events.attr_first(
                    EVENT_TYPE_CONTRACT_INSTANTIATE,
                    EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V2,
                )
            })?
            .value()
            .to_string();

        let contract_address = self.querier.chain_config.parse_address(&contract_address)?;

        Ok((contract_address, resp))
    }

    pub async fn contract_instantiate2(
        &self,
        admin: impl Into<Option<Address>>,
        code_id: u64,
        label: impl ToString,
        msg: &impl Serialize,
        funds: Vec<layer_climb_proto::Coin>,
        salt: Vec<u8>,
        fix_msg: bool,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<(Address, layer_climb_proto::abci::TxResponse)> {
        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&self.contract_instantiate2_msg(
                admin, code_id, label, funds, salt, fix_msg, msg,
            )?)?])
            .await?;

        let events = CosmosTxEvents::from(&resp);

        let contract_address = events
            .attr_first(
                EVENT_TYPE_CONTRACT_INSTANTIATE,
                EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V1,
            )
            .or_else(|_| {
                events.attr_first(
                    EVENT_TYPE_CONTRACT_INSTANTIATE,
                    EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V2,
                )
            })?
            .value()
            .to_string();

        let contract_address = self.querier.chain_config.parse_address(&contract_address)?;

        Ok((contract_address, resp))
    }

    pub async fn contract_execute(
        &self,
        address: &Address,
        msg: &impl Serialize,
        funds: Vec<layer_climb_proto::Coin>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(
                &self.contract_execute_msg(address, funds, msg)?,
            )?])
            .await
    }

    pub async fn contract_migrate(
        &self,
        address: &Address,
        code_id: u64,
        msg: &impl Serialize,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(
                &self.contract_migrate_msg(address, code_id, msg)?,
            )?])
            .await
    }
}

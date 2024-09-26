use crate::{
    events::IbcPacket,
    ibc_types::{
        IbcChannelId, IbcChannelOrdering, IbcChannelVersion, IbcClientId, IbcConnectionId,
        IbcPortId,
    },
    prelude::*,
};

// hermes connection handshake: https://github.com/informalsystems/hermes/blob/ccd1d907df4853203349057bba200077254bb83d/crates/relayer/src/connection.rs#L566
// ibc-go connection handshake:
impl SigningClient {
    // this is used for creating a cross-chain client
    // so we need the other chain's client and consensus state
    pub async fn ibc_create_client(
        &self,
        remote_querier: &QueryClient,
        trusting_period_secs: Option<u64>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_create_client_msg(trusting_period_secs, remote_querier)
            .await?;

        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await
    }

    pub async fn ibc_update_client(
        &self,
        client_id: &IbcClientId,
        remote_querier: &QueryClient,
        trusted_height: Option<layer_climb_proto::RevisionHeight>,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = proto_into_any(
            &self
                .ibc_update_client_msg(client_id, remote_querier, trusted_height)
                .await?,
        )?;

        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([msg])
            .await
    }

    pub async fn ibc_open_connection_init(
        &self,
        client_id: &IbcClientId,
        counterparty_client_id: &IbcClientId,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = proto_into_any(
            &self
                .ibc_open_connection_init_msg(client_id, counterparty_client_id)
                .await?,
        )?;

        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([msg])
            .await;

        // wait 1 block so client update height - 1 will see it
        self.querier.wait_blocks(1, None).await?;

        resp
    }

    pub async fn ibc_open_connection_try(
        &self,
        client_id: &IbcClientId,
        counterparty_client_id: &IbcClientId,
        counterparty_connection_id: &IbcConnectionId,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_open_connection_try_msg(
                client_id,
                counterparty_client_id,
                counterparty_connection_id,
                remote_querier,
            )
            .await?;

        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await;

        // wait 1 block so client update height - 1 will see it
        self.querier.wait_blocks(1, None).await?;

        resp
    }

    pub async fn ibc_open_connection_ack(
        &self,
        client_id: &IbcClientId,
        counterparty_client_id: &IbcClientId,
        connection_id: &IbcConnectionId,
        counterparty_connection_id: &IbcConnectionId,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_open_connection_ack_msg(
                client_id,
                counterparty_client_id,
                connection_id,
                counterparty_connection_id,
                remote_querier,
            )
            .await?;

        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await;

        // wait 1 block so client update height - 1 will see it
        self.querier.wait_blocks(1, None).await?;

        resp
    }

    pub async fn ibc_open_connection_confirm(
        &self,
        client_id: &IbcClientId,
        counterparty_client_id: &IbcClientId,
        connection_id: &IbcConnectionId,
        counterparty_connection_id: &IbcConnectionId,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_open_connection_confirm_msg(
                client_id,
                counterparty_client_id,
                connection_id,
                counterparty_connection_id,
                remote_querier,
            )
            .await?;

        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await
    }

    pub async fn ibc_open_channel_init(
        &self,
        connection_id: &IbcConnectionId,
        port_id: &IbcPortId,
        version: &IbcChannelVersion,
        ordering: IbcChannelOrdering,
        counterparty_port_id: &IbcPortId,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self.ibc_open_channel_init_msg(
            connection_id,
            port_id,
            version,
            ordering,
            counterparty_port_id,
        )?;

        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await;

        // wait 1 block so client update height - 1 will see it
        self.querier.wait_blocks(1, None).await?;

        resp
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn ibc_open_channel_try(
        &self,
        client_id: &IbcClientId,
        connection_id: &IbcConnectionId,
        port_id: &IbcPortId,
        version: &IbcChannelVersion,
        counterparty_port_id: &IbcPortId,
        counterparty_channel_id: &IbcChannelId,
        counterparty_version: &IbcChannelVersion,
        ordering: IbcChannelOrdering,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_open_channel_try_msg(
                client_id,
                connection_id,
                port_id,
                version,
                counterparty_port_id,
                counterparty_channel_id,
                counterparty_version,
                ordering,
                remote_querier,
            )
            .await?;

        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await;

        // wait 1 block so client update height - 1 will see it
        self.querier.wait_blocks(1, None).await?;

        resp
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn ibc_open_channel_ack(
        &self,
        client_id: &IbcClientId,
        channel_id: &IbcChannelId,
        port_id: &IbcPortId,
        counterparty_port_id: &IbcPortId,
        counterparty_channel_id: &IbcChannelId,
        counterparty_version: &IbcChannelVersion,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_open_channel_ack_msg(
                client_id,
                channel_id,
                port_id,
                counterparty_port_id,
                counterparty_channel_id,
                counterparty_version,
                remote_querier,
            )
            .await?;

        let resp = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await;

        // wait 1 block so client update height - 1 will see it
        self.querier.wait_blocks(1, None).await?;

        resp
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn ibc_open_channel_confirm(
        &self,
        client_id: &IbcClientId,
        channel_id: &IbcChannelId,
        port_id: &IbcPortId,
        counterparty_port_id: &IbcPortId,
        counterparty_channel_id: &IbcChannelId,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_open_channel_confirm_msg(
                client_id,
                channel_id,
                port_id,
                counterparty_port_id,
                counterparty_channel_id,
                remote_querier,
            )
            .await?;

        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await
    }

    // the querier is where the packet arrived *from*
    // this should be called on the chain the packet is being sent *to*
    pub async fn ibc_packet_recv(
        &self,
        client_id: &IbcClientId,
        packet: IbcPacket,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_packet_recv_msg(client_id, packet, remote_querier)
            .await?;

        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await
    }

    // the querier is where the packet arrived *from*
    // this should be called on the chain the packet is being sent *to*
    pub async fn ibc_packet_ack(
        &self,
        client_id: &IbcClientId,
        packet: IbcPacket,
        remote_querier: &QueryClient,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let msg = self
            .ibc_packet_ack_msg(client_id, packet, remote_querier)
            .await?;

        tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(&msg)?])
            .await
    }
}

use crate::{
    events::{
        EVENT_ATTR_IBC_CHANNEL_ID, EVENT_ATTR_IBC_CONNECTION_ID, EVENT_TYPE_IBC_CHANNEL_OPEN_INIT,
        EVENT_TYPE_IBC_CHANNEL_OPEN_TRY, EVENT_TYPE_IBC_CONNECTION_OPEN_INIT,
        EVENT_TYPE_IBC_CONNECTION_OPEN_TRY, EVENT_TYPE_IBC_CREATE_CLIENT,
    },
    ibc_types::{
        IbcChannelId, IbcChannelOrdering, IbcChannelVersion, IbcClientId, IbcConnectionId,
        IbcPortId,
    },
    prelude::*,
};
use anyhow::ensure;
use serde::{Deserialize, Serialize};

impl SigningClient {
    pub async fn ibc_connection_handshake(
        &self,
        counterparty_client: &SigningClient,
        client_id: Option<IbcClientId>,
        counterparty_client_id: Option<IbcClientId>,
        // if None, IbcConnectionHandshakeGasSimulationMultipliers::default() will be used
        simulation_gas_multipliers: Option<IbcConnectionHandshakeGasSimulationMultipliers>,
        logger: impl Fn(String),
    ) -> Result<IbcConnectionHandshake> {
        macro_rules! write_out {
            ($($arg:tt)*) => {
                logger(format!($($arg)*));
            };
        }
        let client_id = match client_id {
            Some(id) => id,
            None => {
                write_out!("No client ID for network 1, creating one");
                let mut tx_builder = self.tx_builder();
                if let Some(gas_multiplier) =
                    simulation_gas_multipliers.as_ref().and_then(|m| m.client_1)
                {
                    tx_builder.set_gas_simulate_multiplier(gas_multiplier);
                }
                let tx_resp = self
                    .ibc_create_client(&counterparty_client.querier, None, Some(tx_builder))
                    .await?;
                let events = CosmosTxEvents::from(&tx_resp);
                let event = events
                    .event_first_by_type(EVENT_TYPE_IBC_CREATE_CLIENT)
                    .context("No create_client event found")?;
                let attr = event
                    .attributes()
                    .find(|attr| attr.key() == "client_id")
                    .context("No client_id attribute found")?;
                let client_id = IbcClientId::new(attr.value().to_string());
                write_out!("client ID for network 1 is {}", client_id);
                client_id
            }
        };

        let counterparty_client_id = match counterparty_client_id {
            Some(id) => id,
            None => {
                write_out!("No client ID for network 2, creating one");
                let mut tx_builder = counterparty_client.tx_builder();
                if let Some(gas_multiplier) =
                    simulation_gas_multipliers.as_ref().and_then(|m| m.client_2)
                {
                    tx_builder.set_gas_simulate_multiplier(gas_multiplier);
                }
                let tx_resp = counterparty_client
                    .ibc_create_client(&self.querier, None, Some(tx_builder))
                    .await?;
                let events = CosmosTxEvents::from(&tx_resp);
                let event = events
                    .event_first_by_type(EVENT_TYPE_IBC_CREATE_CLIENT)
                    .context("No create_client event found")?;
                let attr = event
                    .attributes()
                    .find(|attr| attr.key() == "client_id")
                    .context("No client_id attribute found")?;
                let client_id = IbcClientId::new(attr.value().to_string());
                write_out!("client ID for network 2 is {}", client_id);
                client_id
            }
        };

        // connection init
        let connection_id = {
            write_out!("[CONNECTION INIT] starting on chain {}", self.chain_id());
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.connection_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            let tx_resp = self
                .ibc_open_connection_init(&client_id, &counterparty_client_id, Some(tx_builder))
                .await?;
            let events = CosmosTxEvents::from(&tx_resp);
            // https://github.com/cosmos/ibc-go/blob/d771177acf66890c9c6f6e5df9a37b8031dbef7d/modules/core/03-connection/types/events.go#L19
            let connection_id = events
                .attr_first(
                    EVENT_TYPE_IBC_CONNECTION_OPEN_INIT,
                    EVENT_ATTR_IBC_CONNECTION_ID,
                )?
                .value()
                .to_string();
            let connection_id = IbcConnectionId::new(connection_id);
            write_out!(
                "[CONNECTION INIT] on chain {}, connection id: {}",
                self.chain_id(),
                connection_id
            );
            connection_id
        };

        // update clients
        {
            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                self.chain_id(),
                client_id,
                counterparty_client.chain_id(),
                counterparty_client_id
            );
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_update_client(
                &client_id,
                &counterparty_client.querier,
                None,
                Some(tx_builder),
            )
            .await?;

            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                counterparty_client.chain_id(),
                counterparty_client_id,
                self.chain_id(),
                client_id
            );
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_update_client(
                    &counterparty_client_id,
                    &self.querier,
                    None,
                    Some(tx_builder),
                )
                .await?;
            write_out!("[CLIENTS UPDATED]");
        }

        // connection try
        let counterparty_connection_id = {
            write_out!(
                "[CONNECTION TRY] starting on chain {}, connection_id: {}",
                counterparty_client.chain_id(),
                connection_id
            );
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.connection_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            let tx_resp = counterparty_client
                .ibc_open_connection_try(
                    &counterparty_client_id,
                    &client_id,
                    &connection_id,
                    &self.querier,
                    Some(tx_builder),
                )
                .await?;
            let events = CosmosTxEvents::from(&tx_resp);
            let counterparty_connection_id = events
                .attr_first(
                    EVENT_TYPE_IBC_CONNECTION_OPEN_TRY,
                    EVENT_ATTR_IBC_CONNECTION_ID,
                )?
                .value()
                .to_string();
            let counterparty_connection_id = IbcConnectionId::new(counterparty_connection_id);
            write_out!("[CONNECTION TRY] completed on chain {}, src connection_id: {}, dst connection_id: {}", counterparty_client.chain_id(), counterparty_connection_id, connection_id);
            counterparty_connection_id
        };

        // update clients
        {
            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                self.chain_id(),
                client_id,
                counterparty_client.chain_id(),
                counterparty_client_id
            );
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_update_client(
                &client_id,
                &counterparty_client.querier,
                None,
                Some(tx_builder),
            )
            .await?;

            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                counterparty_client.chain_id(),
                counterparty_client_id,
                self.chain_id(),
                client_id
            );
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_update_client(
                    &counterparty_client_id,
                    &self.querier,
                    None,
                    Some(tx_builder),
                )
                .await?;
            write_out!("[CLIENTS UPDATED]");
        }

        // connection ack
        {
            write_out!("[CONNECTION ACK] starting on chain {}, src connection_id: {}, dst connection_id: {}", self.chain_id(), connection_id, counterparty_connection_id);
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.connection_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_open_connection_ack(
                &client_id,
                &counterparty_client_id,
                &connection_id,
                &counterparty_connection_id,
                &counterparty_client.querier,
                Some(tx_builder),
            )
            .await?;
            write_out!("[CONNECTION ACK] completed on chain {}, src connection_id: {}, dst connection_id: {}", self.chain_id(), connection_id, counterparty_connection_id);
        };

        // update clients
        {
            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                self.chain_id(),
                client_id,
                counterparty_client.chain_id(),
                counterparty_client_id
            );
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_update_client(
                &client_id,
                &counterparty_client.querier,
                None,
                Some(tx_builder),
            )
            .await?;

            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                counterparty_client.chain_id(),
                counterparty_client_id,
                self.chain_id(),
                client_id
            );
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_update_client(
                    &counterparty_client_id,
                    &self.querier,
                    None,
                    Some(tx_builder),
                )
                .await?;
            write_out!("[CLIENTS UPDATED]");
        }

        // connection confirm
        {
            write_out!("[CONNECTION CONFIRM] starting on chain {}, src connection_id: {}, dst connection_id: {}", counterparty_client.chain_id(), counterparty_connection_id, connection_id);
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.connection_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_open_connection_confirm(
                    &counterparty_client_id,
                    &client_id,
                    &counterparty_connection_id,
                    &connection_id,
                    &self.querier,
                    Some(tx_builder),
                )
                .await?;
            write_out!("[CONNECTION CONFIRM] completed on chain {}, src connection_id: {}, dst connection_id: {}", counterparty_client.chain_id(), counterparty_connection_id, connection_id);
        };

        let connection = self.querier.ibc_connection(&connection_id, None).await?;
        ensure!(
            connection.state() == layer_climb_proto::ibc::connection::State::Open,
            "connection state on {} is not {:?} instead it's {:?}",
            self.querier.chain_config.chain_id,
            layer_climb_proto::ibc::connection::State::Open,
            connection.state()
        );
        let counterparty_connection = counterparty_client
            .querier
            .ibc_connection(&counterparty_connection_id, None)
            .await?;
        ensure!(
            counterparty_connection.state() == layer_climb_proto::ibc::connection::State::Open,
            "connection state on {} is not {:?} instead it's {:?}",
            counterparty_client.querier.chain_config.chain_id,
            layer_climb_proto::ibc::connection::State::Open,
            counterparty_connection.state()
        );

        Ok(IbcConnectionHandshake {
            client_id,
            counterparty_client_id,
            connection_id,
            counterparty_connection_id,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn ibc_channel_handshake(
        &self,
        counterparty_client: &SigningClient,
        port_id: &IbcPortId,
        counterparty_port_id: &IbcPortId,
        version: &IbcChannelVersion,
        ordering: IbcChannelOrdering,
        conn_handshake: &IbcConnectionHandshake,
        // if None, IbcChannelHandshakeGasSimulationMultipliers::default() will be used
        simulation_gas_multipliers: Option<IbcChannelHandshakeGasSimulationMultipliers>,
        logger: impl Fn(String),
    ) -> Result<IbcChannelHandshake> {
        macro_rules! write_out {
            ($($arg:tt)*) => {
                logger(format!($($arg)*));
            };
        }

        let IbcConnectionHandshake {
            client_id,
            counterparty_client_id,
            connection_id,
            counterparty_connection_id,
        } = conn_handshake;

        // channel init
        let channel_id = {
            write_out!("[CHANNEL INIT] starting on chain {}", self.chain_id());

            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.channel_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            let tx_resp = self
                .ibc_open_channel_init(
                    connection_id,
                    port_id,
                    version,
                    ordering,
                    counterparty_port_id,
                    Some(tx_builder),
                )
                .await?;

            let events = CosmosTxEvents::from(&tx_resp);
            let channel_id = events
                .attr_first(EVENT_TYPE_IBC_CHANNEL_OPEN_INIT, EVENT_ATTR_IBC_CHANNEL_ID)?
                .value()
                .to_string();
            let channel_id = IbcChannelId::new(channel_id);
            write_out!(
                "[CHANNEL INIT] completed on chain {}, channel id: {}",
                self.chain_id(),
                channel_id
            );
            channel_id
        };

        // update clients
        {
            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                self.chain_id(),
                client_id,
                counterparty_client.chain_id(),
                counterparty_client_id
            );
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_update_client(
                client_id,
                &counterparty_client.querier,
                None,
                Some(tx_builder),
            )
            .await?;

            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                counterparty_client.chain_id(),
                counterparty_client_id,
                self.chain_id(),
                client_id
            );
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_update_client(
                    counterparty_client_id,
                    &self.querier,
                    None,
                    Some(tx_builder),
                )
                .await?;
            write_out!("[CLIENTS UPDATED]");
        }

        let counterparty_channel_id = {
            write_out!(
                "[CHANNEL TRY] starting on chain {}, channel_id: {}",
                counterparty_client.chain_id(),
                channel_id
            );

            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.channel_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            let tx_resp = counterparty_client
                .ibc_open_channel_try(
                    counterparty_client_id,
                    counterparty_connection_id,
                    counterparty_port_id,
                    version,
                    port_id,
                    &channel_id,
                    version,
                    ordering,
                    &self.querier,
                    Some(tx_builder),
                )
                .await?;

            let events = CosmosTxEvents::from(&tx_resp);
            let counterparty_channel_id = events
                .attr_first(EVENT_TYPE_IBC_CHANNEL_OPEN_TRY, EVENT_ATTR_IBC_CHANNEL_ID)?
                .value()
                .to_string();
            let counterparty_channel_id = IbcChannelId::new(counterparty_channel_id);
            write_out!(
                "[CHANNEL TRY] completed on chain {}, src channel_id: {}, dest channel id: {}",
                counterparty_client.chain_id(),
                counterparty_channel_id,
                channel_id
            );
            counterparty_channel_id
        };

        // update clients
        {
            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                self.chain_id(),
                client_id,
                counterparty_client.chain_id(),
                counterparty_client_id
            );
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_update_client(
                client_id,
                &counterparty_client.querier,
                None,
                Some(tx_builder),
            )
            .await?;

            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                counterparty_client.chain_id(),
                counterparty_client_id,
                self.chain_id(),
                client_id
            );
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_update_client(
                    counterparty_client_id,
                    &self.querier,
                    None,
                    Some(tx_builder),
                )
                .await?;
            write_out!("[CLIENTS UPDATED]");
        }

        {
            write_out!(
                "[CHANNEL ACK] starting on chain {}, src channel_id: {}, dst channel_id: {}",
                self.chain_id(),
                channel_id,
                counterparty_channel_id
            );

            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.channel_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_open_channel_ack(
                client_id,
                &channel_id,
                port_id,
                counterparty_port_id,
                &counterparty_channel_id,
                version,
                &counterparty_client.querier,
                Some(tx_builder),
            )
            .await?;
            write_out!(
                "[CHANNEL ACK] completed on chain {}, src channel_id: {}, dst channel_id: {}",
                self.chain_id(),
                channel_id,
                counterparty_channel_id
            );
        };

        // update clients
        {
            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                self.chain_id(),
                client_id,
                counterparty_client.chain_id(),
                counterparty_client_id
            );
            let mut tx_builder = self.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_1)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            self.ibc_update_client(
                client_id,
                &counterparty_client.querier,
                None,
                Some(tx_builder),
            )
            .await?;

            write_out!(
                "[CLIENT UPDATE] starting {}:{} -> {}:{}",
                counterparty_client.chain_id(),
                counterparty_client_id,
                self.chain_id(),
                client_id
            );
            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.update_client_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_update_client(
                    counterparty_client_id,
                    &self.querier,
                    None,
                    Some(tx_builder),
                )
                .await?;
            write_out!("[CLIENTS UPDATED]");
        }

        {
            write_out!(
                "[CHANNEL CONFIRM] starting on chain {}, src channel_id: {}, dst channel_id: {}",
                counterparty_client.chain_id(),
                counterparty_channel_id,
                channel_id
            );

            let mut tx_builder = counterparty_client.tx_builder();
            if let Some(gas_multiplier) = simulation_gas_multipliers
                .as_ref()
                .and_then(|m| m.channel_2)
            {
                tx_builder.set_gas_simulate_multiplier(gas_multiplier);
            }
            counterparty_client
                .ibc_open_channel_confirm(
                    counterparty_client_id,
                    &counterparty_channel_id,
                    counterparty_port_id,
                    port_id,
                    &channel_id,
                    &self.querier,
                    Some(tx_builder),
                )
                .await?;

            write_out!(
                "[CHANNEL CONFIRM] completed on chain {}, src channel_id: {}, dst channel_id: {}",
                counterparty_client.chain_id(),
                counterparty_channel_id,
                channel_id
            );
        };

        let channel = self.querier.ibc_channel(&channel_id, port_id, None).await?;
        ensure!(
            channel.state() == layer_climb_proto::ibc::channel::State::Open,
            "channel state on {} is not {:?} instead it's {:?}",
            self.querier.chain_config.chain_id,
            layer_climb_proto::ibc::channel::State::Open,
            channel.state()
        );
        let counterparty_channel = counterparty_client
            .querier
            .ibc_channel(&counterparty_channel_id, counterparty_port_id, None)
            .await?;
        ensure!(
            counterparty_channel.state() == layer_climb_proto::ibc::channel::State::Open,
            "channel state on {} is not {:?} instead it's {:?}",
            counterparty_client.querier.chain_config.chain_id,
            layer_climb_proto::ibc::channel::State::Open,
            counterparty_channel.state()
        );

        Ok(IbcChannelHandshake {
            channel_id,
            counterparty_channel_id,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbcConnectionHandshake {
    pub client_id: IbcClientId,
    pub counterparty_client_id: IbcClientId,
    pub connection_id: IbcConnectionId,
    pub counterparty_connection_id: IbcConnectionId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbcChannelHandshake {
    pub channel_id: IbcChannelId,
    pub counterparty_channel_id: IbcChannelId,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IbcConnectionHandshakeGasSimulationMultipliers {
    pub client_1: Option<f32>,
    pub client_2: Option<f32>,
    pub connection_1: Option<f32>,
    pub connection_2: Option<f32>,
    pub update_client_1: Option<f32>,
    pub update_client_2: Option<f32>,
}

impl Default for IbcConnectionHandshakeGasSimulationMultipliers {
    fn default() -> Self {
        Self {
            client_1: Some(2.5),
            client_2: Some(2.5),
            connection_1: Some(2.5),
            connection_2: Some(2.5),
            update_client_1: Some(2.5),
            update_client_2: Some(2.5),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IbcChannelHandshakeGasSimulationMultipliers {
    pub channel_1: Option<f32>,
    pub channel_2: Option<f32>,
    pub update_client_1: Option<f32>,
    pub update_client_2: Option<f32>,
}

impl Default for IbcChannelHandshakeGasSimulationMultipliers {
    fn default() -> Self {
        Self {
            channel_1: Some(2.5),
            channel_2: Some(2.5),
            update_client_1: Some(2.5),
            update_client_2: Some(2.5),
        }
    }
}

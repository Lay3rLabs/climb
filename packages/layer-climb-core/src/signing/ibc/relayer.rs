/*
High-level overview of the relayer:

RELAYER

1. It will listen for IBC packets on all chains
2. There are two separate tasks: one for producing tasks (listening for IBC packets) and one for consuming tasks (updating clients and relaying packets)
3. These are not run in parallel, but rather in a join, so that it works in a single-threaded environment like browsers
4. Also, the task sender does _very little_ work, just formats the tasks, so it doesn't block the event loop anyway

CACHE

1. "prepping the cache" creates all the clients, connections, and channels that the relayer will use
2. it will automatically try to update all clients and invalidate its cache as needed
3. basically, that means you can just "prep the cache" with the last prepped-cache and everything will work as expected

*/
mod builder;
pub use builder::*;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{
    events::{IbcPacket, IbcPacketKind},
    ibc_types::{
        IbcChannelId, IbcChannelOrdering, IbcChannelVersion, IbcClientId, IbcConnectionId,
        IbcPortId,
    },
    prelude::*,
    querier::stream::BlockEvents,
};
use futures::StreamExt;

use serde::{Deserialize, Serialize};

use super::{
    IbcChannelHandshakeGasSimulationMultipliers, IbcConnectionHandshakeGasSimulationMultipliers,
};

pub struct IbcRelayer {
    simulation_gas_multipliers: IbcRelayerGasSimulationMultipliers,
    inner_log_ok: Arc<dyn Fn(String) + Send + Sync + 'static>,
    inner_log_err: Arc<dyn Fn(String) + Send + Sync + 'static>,
    client_infos: Vec<Arc<ClientInfo>>,
}

impl IbcRelayer {
    pub async fn start(&self) -> Result<()> {
        // at a high-level, we're streaming events in as they come in and kicking off tasks to handle them
        let (task_sender, task_receiver) = futures::channel::mpsc::unbounded();

        let resp = futures::future::join(
            self.produce_tasks(task_sender),
            self.consume_tasks(task_receiver),
        )
        .await;

        match resp {
            (Ok(()), _) => Ok(()),
            (Err(e), _) => Err(e),
        }
    }

    async fn produce_tasks(
        &self,
        task_sender: futures::channel::mpsc::UnboundedSender<Task>,
    ) -> Result<()> {
        // get a single stream for each client, regardless of whether it's side one or two
        let mut unique_clients = HashMap::new();

        for client_info in self.client_infos.iter() {
            let chain_id_1 = client_info.signing_client_1.chain_id();
            let chain_id_2 = client_info.signing_client_2.chain_id();

            if !unique_clients.contains_key(chain_id_1) {
                unique_clients.insert(
                    chain_id_1.clone(),
                    client_info.signing_client_1.querier.clone(),
                );
            }

            if !unique_clients.contains_key(chain_id_2) {
                unique_clients.insert(
                    chain_id_2.clone(),
                    client_info.signing_client_2.querier.clone(),
                );
            }
        }

        let mut streams = Vec::new();

        for (chain_id, client) in unique_clients.iter() {
            let stream = Box::pin(
                client
                    .clone()
                    .stream_block_events(None)
                    .await?
                    .map(move |events| (chain_id, events)),
            );

            streams.push(stream);
        }

        // with all the streams combined, we can now select on them and process each event as it comes in
        let mut combined_stream = futures::stream::select_all(streams);

        while let Some((chain_id, events)) = combined_stream.next().await {
            match events {
                Ok(events) => {
                    // encapsulate so we can log errors instead of die
                    match self
                        .produce_event_tasks(chain_id, events, &task_sender)
                        .await
                    {
                        Ok(()) => {}
                        Err(e) => {
                            self.log_err(format!(
                                "Error processing events for chain {chain_id}: {e:?}"
                            ));
                        }
                    }
                }
                Err(e) => {
                    self.log_err(format!("Error querying chain {chain_id}: {e:?}"));
                }
            }
        }

        Ok(())
    }

    async fn consume_tasks(
        &self,
        mut task_receiver: futures::channel::mpsc::UnboundedReceiver<Task>,
    ) {
        while let Some(task) = task_receiver.next().await {
            // encapsulate so we can log errors instead of die
            match self.consume_task(task).await {
                Ok(()) => {}
                Err(e) => {
                    self.log_err(format!("Error handling task: {e:?}"));
                }
            }
        }
    }

    // the main event loop for producing tasks
    // this should be very quick and not block, as much as possible
    // heavy lifting is done in the task itself
    async fn produce_event_tasks(
        &self,
        chain_id: &ChainId,
        block_events: BlockEvents,
        task_sender: &futures::channel::mpsc::UnboundedSender<Task>,
    ) -> Result<()> {
        macro_rules! write_out {
            ($($arg:tt)*) => {
                self.log_ok(format!($($arg)*));
            };
        }

        let BlockEvents { height, events } = block_events;

        #[allow(clippy::collapsible_if)]
        for client_info in self.client_infos.iter() {
            if client_info.signing_client_1.chain_id() == chain_id {
                if !client_info.update_1.get_is_auto_updating()
                    && client_info.is_past_update_height(Side::One, height).await?
                {
                    client_info.update_1.set_is_auto_updating(true);
                    task_sender.unbounded_send(Task::AutoUpdateClient {
                        client_info: client_info.clone(),
                        side: Side::One,
                    })?;
                }
            } else if client_info.signing_client_2.chain_id() == chain_id {
                if !client_info.update_2.get_is_auto_updating()
                    && client_info.is_past_update_height(Side::Two, height).await?
                {
                    client_info.update_2.set_is_auto_updating(true);
                    task_sender.unbounded_send(Task::AutoUpdateClient {
                        client_info: client_info.clone(),
                        side: Side::Two,
                    })?;
                }
            }
        }

        let events = CosmosTxEvents::from(events.as_slice());

        for event in events.events_iter() {
            match IbcPacket::try_from(&event) {
                Ok(packet) => {
                    write_out!("[IBC EVENT] {:?}", packet.kind);
                    task_sender.unbounded_send(Task::RelayPacket {
                        client_packet: Box::new(
                            self.get_client_packet(chain_id, packet)?
                                .context("couldn't find client info for packet")?,
                        ),
                    })?;
                }
                Err(_) => {
                    // non-ibc-event
                }
            }
        }
        Ok(())
    }

    // the main workhose for consuming tasks
    async fn consume_task(&self, task: Task) -> Result<()> {
        macro_rules! write_out {
            ($($arg:tt)*) => {
                self.log_ok(format!($($arg)*));
            };
        }
        match task {
            Task::AutoUpdateClient { client_info, side } => {
                self.update_ibc_client(&client_info, side).await?;
            }
            Task::RelayPacket { client_packet } => {
                let ClientPacket {
                    client_info,
                    side,
                    packet,
                } = *client_packet;

                match packet.kind {
                    IbcPacketKind::Send | IbcPacketKind::WriteAck => {
                        // always write to the chain opposite the event source
                        let (dst_signing_client, src_querier, dst_ibc_client_id, tx_builder) =
                            match side {
                                Side::One => (
                                    client_info.signing_client_2.clone(),
                                    client_info.signing_client_1.querier.clone(),
                                    client_info.ibc_client_id_2.clone(),
                                    client_info
                                        .tx_builder(Side::Two, &self.simulation_gas_multipliers),
                                ),
                                Side::Two => (
                                    client_info.signing_client_1.clone(),
                                    client_info.signing_client_2.querier.clone(),
                                    client_info.ibc_client_id_1.clone(),
                                    client_info
                                        .tx_builder(Side::One, &self.simulation_gas_multipliers),
                                ),
                            };

                        // not sure why the order matters, change at your own peril
                        // also, you might think you can skip over updates if it's recent enough.
                        // maybe, good luck :)
                        match side {
                            Side::One => {
                                self.update_ibc_client(&client_info, Side::One).await?;
                                self.update_ibc_client(&client_info, Side::Two).await?;
                            }
                            Side::Two => {
                                self.update_ibc_client(&client_info, Side::Two).await?;
                                self.update_ibc_client(&client_info, Side::One).await?;
                            }
                        }

                        if packet.kind == IbcPacketKind::Send {
                            write_out!(
                                "[RELAYING PACKET SEND] {}:{} -> {}:{}",
                                src_querier.chain_config.chain_id,
                                packet.src_port_id,
                                dst_signing_client.chain_id(),
                                packet.dst_port_id
                            );
                            dst_signing_client
                                .ibc_packet_recv(
                                    &dst_ibc_client_id,
                                    packet,
                                    &src_querier,
                                    Some(tx_builder),
                                )
                                .await?;
                        } else if packet.kind == IbcPacketKind::WriteAck {
                            write_out!(
                                "[RELAYING PACKET ACK] {}:{} -> {}:{}",
                                src_querier.chain_config.chain_id,
                                packet.src_port_id,
                                dst_signing_client.chain_id(),
                                packet.dst_port_id
                            );
                            dst_signing_client
                                .ibc_packet_ack(
                                    &dst_ibc_client_id,
                                    packet.clone(),
                                    &src_querier,
                                    Some(tx_builder),
                                )
                                .await?;
                        }
                    }
                    IbcPacketKind::Ack => {
                        write_out!(
                            "[PACKET ACK] CONFIRMED {} <-> {}",
                            packet.src_port_id,
                            packet.dst_port_id
                        );
                    }
                    IbcPacketKind::Timeout => {
                        // TODO - handle timeouts?
                        write_out!(
                            "[PACKET TIMEOUT] {} <-> {}",
                            packet.src_port_id,
                            packet.dst_port_id
                        );
                    }
                    IbcPacketKind::Receive => {
                        // TODO - handle receives?
                        write_out!(
                            "[PACKET RECEIVE] {} <-> {}",
                            packet.src_port_id,
                            packet.dst_port_id
                        );
                    }
                }
            }
        }
        Ok(())
    }

    async fn update_ibc_client(&self, client_info: &ClientInfo, side: Side) -> Result<()> {
        let log_ok = self.inner_log_ok.clone();
        client_info
            .update(side, &self.simulation_gas_multipliers, move |s| log_ok(s))
            .await
    }

    // get the client info for a given chain and packet
    // also normalizes the packet so that it always points from src->dst
    // from the perspective of the chain the event was detected on
    fn get_client_packet(
        &self,
        chain_id: &ChainId,
        mut packet: IbcPacket,
    ) -> Result<Option<ClientPacket>> {
        for client_info in self.client_infos.iter() {
            let side = if chain_id == client_info.signing_client_1.chain_id() {
                Some(Side::One)
            } else if chain_id == client_info.signing_client_2.chain_id() {
                Some(Side::Two)
            } else {
                None
            };

            // at this point, the packet is from the event, and src_connection_id is identical to dst_connection_id, doesn't matter which we check
            if side.is_none()
                || (packet.src_connection_id != client_info.connection_id_1
                    && packet.src_connection_id != client_info.connection_id_2)
            {
                continue;
            }

            let side = side.unwrap();

            for channel in client_info.channels.iter() {
                if packet.src_channel_id == channel.channel_id_1
                    && packet.dst_channel_id == channel.channel_id_2
                    && packet.src_port_id == channel.port_id_1
                    && packet.dst_port_id == channel.port_id_2
                {
                    // normalize the packet
                    match side {
                        Side::One => {
                            packet.src_connection_id = client_info.connection_id_1.clone();
                            packet.dst_connection_id = client_info.connection_id_2.clone();
                            // no need to swap channel and port, already in the right order
                        }
                        Side::Two => {
                            packet.src_connection_id = client_info.connection_id_2.clone();
                            packet.dst_connection_id = client_info.connection_id_1.clone();
                            std::mem::swap(&mut packet.src_port_id, &mut packet.dst_port_id);
                            std::mem::swap(&mut packet.src_channel_id, &mut packet.dst_channel_id);
                        }
                    }
                    return Ok(Some(ClientPacket {
                        client_info: client_info.clone(),
                        side,
                        packet,
                    }));
                } else if packet.src_channel_id == channel.channel_id_2
                    && packet.dst_channel_id == channel.channel_id_1
                    && packet.src_port_id == channel.port_id_2
                    && packet.dst_port_id == channel.port_id_1
                {
                    // normalize the packet
                    match side {
                        Side::One => {
                            packet.src_connection_id = client_info.connection_id_1.clone();
                            packet.dst_connection_id = client_info.connection_id_2.clone();
                            std::mem::swap(&mut packet.src_port_id, &mut packet.dst_port_id);
                            std::mem::swap(&mut packet.src_channel_id, &mut packet.dst_channel_id);
                        }
                        Side::Two => {
                            packet.src_connection_id = client_info.connection_id_2.clone();
                            packet.dst_connection_id = client_info.connection_id_1.clone();
                            // no need to swap channel and port, already in the right order
                        }
                    }
                    return Ok(Some(ClientPacket {
                        client_info: client_info.clone(),
                        side,
                        packet,
                    }));
                }
            }
        }
        Ok(None)
    }

    fn log_ok(&self, s: String) {
        (self.inner_log_ok)(s);
    }

    fn log_err(&self, s: String) {
        (self.inner_log_err)(s);
    }
}

enum Task {
    AutoUpdateClient {
        client_info: Arc<ClientInfo>,
        side: Side,
    },
    RelayPacket {
        client_packet: Box<ClientPacket>,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Side {
    One,
    Two,
}
// unique clients for each network
struct ClientInfo {
    pub signing_client_1: SigningClient,
    pub signing_client_2: SigningClient,
    pub ibc_client_id_1: IbcClientId,
    pub ibc_client_id_2: IbcClientId,
    pub trusting_period_1: Duration,
    pub trusting_period_2: Duration,
    pub update_1: ClientUpdate,
    pub update_2: ClientUpdate,
    pub connection_id_1: IbcConnectionId,
    pub connection_id_2: IbcConnectionId,
    pub channels: Vec<ClientInfoChannel>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientInfoChannel {
    pub channel_id_1: IbcChannelId,
    pub channel_id_2: IbcChannelId,
    pub port_id_1: IbcPortId,
    pub port_id_2: IbcPortId,
    pub channel_version: IbcChannelVersion,
    pub channel_ordering: IbcChannelOrdering,
}

impl ClientInfo {
    fn signing_client(&self, side: Side) -> &SigningClient {
        match side {
            Side::One => &self.signing_client_1,
            Side::Two => &self.signing_client_2,
        }
    }

    fn tx_builder(
        &self,
        side: Side,
        simulation_gas_multipliers: &IbcRelayerGasSimulationMultipliers,
    ) -> TxBuilder {
        let mut tx_builder = self.signing_client(side).tx_builder();
        match side {
            Side::One => {
                if let Some(gas_simulation_multiplier) = simulation_gas_multipliers.update_client_1
                {
                    tx_builder.set_gas_simulate_multiplier(gas_simulation_multiplier);
                }
            }
            Side::Two => {
                if let Some(gas_simulation_multiplier) = simulation_gas_multipliers.update_client_2
                {
                    tx_builder.set_gas_simulate_multiplier(gas_simulation_multiplier);
                }
            }
        }

        tx_builder
    }

    fn counterparty_querier(&self, side: Side) -> &QueryClient {
        match side {
            Side::One => &self.signing_client_2.querier,
            Side::Two => &self.signing_client_1.querier,
        }
    }

    fn ibc_client_id(&self, side: Side) -> &IbcClientId {
        match side {
            Side::One => &self.ibc_client_id_1,
            Side::Two => &self.ibc_client_id_2,
        }
    }

    // 2/3 of trusting period
    fn stale_duration(&self, side: Side) -> Duration {
        match side {
            Side::One => self
                .trusting_period_1
                .checked_div(3)
                .unwrap_or_else(|| Duration::from_secs(0)),
            Side::Two => self
                .trusting_period_2
                .checked_div(3)
                .unwrap_or_else(|| Duration::from_secs(0)),
        }
    }

    async fn set_next_update_time(&self, side: Side) -> Result<()> {
        let stale_duration = self.stale_duration(side);

        let mut update_time = self
            .signing_client(side)
            .querier
            .block_header(None)
            .await?
            .time()
            .context("No block time found")?;
        update_time.seconds += i64::try_from(stale_duration.as_secs())?;
        update_time.nanos += i32::try_from(stale_duration.subsec_nanos())?;

        match side {
            Side::One => {
                self.update_1.set_next_time(update_time);
                self.update_1.set_is_auto_updating(false);
            }
            Side::Two => {
                self.update_2.set_next_time(update_time);
                self.update_2.set_is_auto_updating(false);
            }
        }

        Ok(())
    }

    async fn is_past_update_height(&self, side: Side, height: u64) -> Result<bool> {
        let current_time = self
            .signing_client(side)
            .querier
            .block_header(Some(height))
            .await?
            .time()
            .context("No block time found")?;
        let next_update_time = match side {
            Side::One => self.update_1.get_next_time(),
            Side::Two => self.update_2.get_next_time(),
        };
        Ok(current_time.seconds > next_update_time.seconds
            || (current_time.seconds == next_update_time.seconds
                && current_time.nanos > next_update_time.nanos))
    }

    async fn update(
        &self,
        side: Side,
        simulation_gas_multipliers: &IbcRelayerGasSimulationMultipliers,
        log_ok: impl Fn(String) + Send + Sync + 'static,
    ) -> Result<()> {
        let client = self.signing_client(side);
        let ibc_client_id = self.ibc_client_id(side);
        let counterparty_client_querier = self.counterparty_querier(side);

        let height_before = *client
            .querier
            .ibc_client_state(ibc_client_id, None)
            .await?
            .latest_height
            .as_ref()
            .context("missing latest height")?;

        log_ok(format!(
            "[CLIENT UPDATE] starting {}:{} -> {} height: {}",
            client.chain_id(),
            ibc_client_id,
            counterparty_client_querier.chain_config.chain_id,
            height_before.revision_height,
        ));
        let tx_builder = self.tx_builder(side, simulation_gas_multipliers);
        client
            .ibc_update_client(
                ibc_client_id,
                counterparty_client_querier,
                Some(height_before),
                Some(tx_builder),
            )
            .await?;

        let height_after = *client
            .querier
            .ibc_client_state(ibc_client_id, None)
            .await?
            .latest_height
            .as_ref()
            .context("missing latest height")?;

        log_ok(format!(
            "[CLIENT UPDATED] {}:{} -> {} height: {}",
            client.chain_id(),
            ibc_client_id,
            counterparty_client_querier.chain_config.chain_id,
            height_after.revision_height,
        ));

        self.set_next_update_time(side).await?;

        Ok(())
    }
}

#[derive(Default)]
struct ClientUpdate {
    pub next_time_seconds: AtomicI64,
    pub next_time_subnanos: AtomicI32,
    pub is_auto_updating: AtomicBool,
}

impl ClientUpdate {
    pub fn get_next_time(&self) -> layer_climb_proto::Timestamp {
        layer_climb_proto::Timestamp {
            seconds: self.next_time_seconds.load(Ordering::SeqCst),
            nanos: self.next_time_subnanos.load(Ordering::SeqCst),
        }
    }

    pub fn set_next_time(&self, time: layer_climb_proto::Timestamp) {
        self.next_time_seconds.store(time.seconds, Ordering::SeqCst);
        self.next_time_subnanos.store(time.nanos, Ordering::SeqCst);
    }

    pub fn get_is_auto_updating(&self) -> bool {
        self.is_auto_updating.load(Ordering::SeqCst)
    }

    pub fn set_is_auto_updating(&self, is_updating: bool) {
        self.is_auto_updating.store(is_updating, Ordering::SeqCst);
    }
}

struct ClientPacket {
    client_info: Arc<ClientInfo>,
    side: Side,
    packet: IbcPacket,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IbcRelayerGasSimulationMultipliers {
    // if None, IbcConnectionHandshakeGasSimulationMultipliers::default() will be used
    pub connection_handshake: Option<IbcConnectionHandshakeGasSimulationMultipliers>,
    // if None, IbcChannelHandshakeGasSimulationMultipliers::default() will be used
    pub channel_handshake: Option<IbcChannelHandshakeGasSimulationMultipliers>,
    pub update_client_1: Option<f32>,
    pub update_client_2: Option<f32>,
    pub send_packet_1: Option<f32>,
    pub send_packet_2: Option<f32>,
}

impl Default for IbcRelayerGasSimulationMultipliers {
    fn default() -> Self {
        Self {
            connection_handshake: Some(IbcConnectionHandshakeGasSimulationMultipliers::default()),
            channel_handshake: Some(IbcChannelHandshakeGasSimulationMultipliers::default()),
            update_client_1: Some(2.5),
            update_client_2: Some(2.5),
            send_packet_1: Some(2.5),
            send_packet_2: Some(2.5),
        }
    }
}

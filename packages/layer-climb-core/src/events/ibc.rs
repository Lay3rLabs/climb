use crate::ibc_types::{IbcChannelId, IbcConnectionId, IbcPortId};
use crate::prelude::*;

use crate::events::{
    EVENT_ATTR_IBC_CONNECTION_ID, EVENT_ATTR_IBC_PACKET_ACK_HEX, EVENT_ATTR_IBC_PACKET_DATA_HEX,
    EVENT_ATTR_IBC_PACKET_DST_CHANNEL, EVENT_ATTR_IBC_PACKET_DST_PORT,
    EVENT_ATTR_IBC_PACKET_SEQUENCE, EVENT_ATTR_IBC_PACKET_SRC_CHANNEL,
    EVENT_ATTR_IBC_PACKET_SRC_PORT, EVENT_ATTR_IBC_PACKET_TIMEOUT_HEIGHT,
    EVENT_ATTR_IBC_PACKET_TIMEOUT_TIMESTAMP,
};

use super::{
    Event, EVENT_TYPE_IBC_ACK_PACKET, EVENT_TYPE_IBC_RECV_PACKET, EVENT_TYPE_IBC_SEND_PACKET,
    EVENT_TYPE_IBC_TIMEOUT_PACKET, EVENT_TYPE_IBC_WRITE_ACK,
};

#[derive(Clone)]
pub struct IbcPacket {
    pub src_port_id: IbcPortId,
    pub src_channel_id: IbcChannelId,
    pub dst_port_id: IbcPortId,
    pub dst_channel_id: IbcChannelId,
    pub src_connection_id: IbcConnectionId,
    pub dst_connection_id: IbcConnectionId,
    pub sequence: u64,
    pub timeout_height: IbcPacketTimeoutHeight,
    pub timeout_timestamp: u64,
    pub data: Option<Vec<u8>>,
    pub ack: Option<Vec<u8>>,
    pub kind: IbcPacketKind,
}

impl std::fmt::Debug for IbcPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // write each field line by line, but hex encode data and ack
        writeln!(f, "src_port_id: {}", self.src_port_id)?;
        writeln!(f, "src_channel_id: {}", self.src_channel_id)?;
        writeln!(f, "dst_port_id: {}", self.dst_port_id)?;
        writeln!(f, "dst_channel_id: {}", self.dst_channel_id)?;
        writeln!(f, "src_connection_id: {}", self.src_connection_id)?;
        writeln!(f, "dst_connection_id: {}", self.dst_connection_id)?;
        writeln!(f, "sequence: {}", self.sequence)?;
        writeln!(f, "timeout_height: {:?}", self.timeout_height)?;
        writeln!(f, "timeout_timestamp: {}", self.timeout_timestamp)?;
        if let Some(data) = &self.data {
            writeln!(f, "data: {}", const_hex::encode(data))?;
        }
        if let Some(ack) = &self.ack {
            writeln!(f, "ack: {}", const_hex::encode(ack))?;
        }
        writeln!(f, "kind: {:?}", self.kind)
    }
}

impl IbcPacket {
    pub fn invert(&mut self) {
        std::mem::swap(&mut self.src_port_id, &mut self.dst_port_id);
        std::mem::swap(&mut self.src_channel_id, &mut self.dst_channel_id);
        std::mem::swap(&mut self.src_connection_id, &mut self.dst_connection_id);
    }
}

#[derive(Clone, Debug)]
pub enum IbcPacketTimeoutHeight {
    None,
    Revision { height: u64, revision: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IbcPacketKind {
    Send,
    Receive,
    WriteAck,
    Ack,
    Timeout,
}

impl TryFrom<&Event<'_>> for IbcPacketKind {
    type Error = anyhow::Error;

    fn try_from(event: &Event) -> Result<Self> {
        if event.is_type(EVENT_TYPE_IBC_SEND_PACKET) {
            Ok(IbcPacketKind::Send)
        } else if event.is_type(EVENT_TYPE_IBC_RECV_PACKET) {
            Ok(IbcPacketKind::Receive)
        } else if event.is_type(EVENT_TYPE_IBC_ACK_PACKET) {
            Ok(IbcPacketKind::Ack)
        } else if event.is_type(EVENT_TYPE_IBC_WRITE_ACK) {
            Ok(IbcPacketKind::WriteAck)
        } else if event.is_type(EVENT_TYPE_IBC_TIMEOUT_PACKET) {
            Ok(IbcPacketKind::Timeout)
        } else {
            Err(anyhow!("not an IBC packet event type: {}", event.ty()))
        }
    }
}

impl<'a> TryFrom<&Event<'a>> for IbcPacket {
    type Error = anyhow::Error;

    fn try_from(event: &Event<'a>) -> Result<Self> {
        let kind: IbcPacketKind = event.try_into()?;

        #[derive(Default)]
        struct IbcPacketBuilder {
            pub src_port_id: Option<IbcPortId>,
            pub src_channel_id: Option<IbcChannelId>,
            pub dst_port_id: Option<IbcPortId>,
            pub dst_channel_id: Option<IbcChannelId>,
            pub connection_id: Option<IbcConnectionId>,
            pub sequence: Option<u64>,
            pub timeout_height: Option<IbcPacketTimeoutHeight>,
            pub timeout_timestamp: Option<u64>,
            pub data: Option<Vec<u8>>,
            pub ack: Option<Vec<u8>>,
        }

        let mut builder = IbcPacketBuilder::default();

        // https://github.com/cosmos/relayer/blob/16a64aaac1839cd799c5b6e99458a644f68d788c/relayer/chains/parsing.go#L216
        for attribute in event.attributes() {
            if attribute.key() == EVENT_ATTR_IBC_PACKET_SRC_PORT {
                builder.src_port_id = Some(IbcPortId::new(attribute.value()));
            }

            if attribute.key() == EVENT_ATTR_IBC_PACKET_SRC_CHANNEL {
                builder.src_channel_id = Some(IbcChannelId::new(attribute.value()));
            }

            if attribute.key() == EVENT_ATTR_IBC_PACKET_DST_PORT {
                builder.dst_port_id = Some(IbcPortId::new(attribute.value()));
            }

            if attribute.key() == EVENT_ATTR_IBC_PACKET_DST_CHANNEL {
                builder.dst_channel_id = Some(IbcChannelId::new(attribute.value()));
            }

            if attribute.key() == EVENT_ATTR_IBC_CONNECTION_ID {
                builder.connection_id = Some(IbcConnectionId::new(attribute.value()));
            }

            if attribute.key() == EVENT_ATTR_IBC_PACKET_SEQUENCE {
                builder.sequence = Some(attribute.value().parse()?);
            }

            if attribute.key() == EVENT_ATTR_IBC_PACKET_TIMEOUT_HEIGHT {
                // "{revision}-{height}"
                let mut s = attribute.value().split('-');
                let revision: u64 = s
                    .next()
                    .ok_or_else(|| anyhow!("missing revision"))?
                    .parse()?;
                let height: u64 = s.next().ok_or_else(|| anyhow!("missing height"))?.parse()?;
                // uggggggh https://github.com/informalsystems/hermes/blob/1ee344fe5be1670fcb629e01b3ccb08c9e1ad9c2/crates/relayer-types/src/core/ics04_channel/timeout.rs#L22
                if revision == 0 && height == 0 {
                    builder.timeout_height = Some(IbcPacketTimeoutHeight::None);
                } else {
                    builder.timeout_height =
                        Some(IbcPacketTimeoutHeight::Revision { revision, height });
                }
            }

            if attribute.key() == EVENT_ATTR_IBC_PACKET_TIMEOUT_TIMESTAMP {
                builder.timeout_timestamp = Some(attribute.value().parse()?);
            }

            if attribute.key() == EVENT_ATTR_IBC_PACKET_DATA_HEX {
                let data = const_hex::decode(attribute.value())?;
                builder.data = Some(data);
            }
            if attribute.key() == EVENT_ATTR_IBC_PACKET_ACK_HEX {
                let ack = const_hex::decode(attribute.value())?;
                builder.ack = Some(ack);
            }
        }

        // connection_id from the event is just one field, doesn't know about src/dst
        // this will be corrected in packet "normalization" in the relayer
        let connection_id = builder
            .connection_id
            .ok_or_else(|| anyhow!("missing connection id"))?;

        Ok(IbcPacket {
            src_port_id: builder
                .src_port_id
                .ok_or_else(|| anyhow!("missing src port"))?,
            src_channel_id: builder
                .src_channel_id
                .ok_or_else(|| anyhow!("missing src channel"))?,
            dst_port_id: builder
                .dst_port_id
                .ok_or_else(|| anyhow!("missing dst port"))?,
            dst_channel_id: builder
                .dst_channel_id
                .ok_or_else(|| anyhow!("missing dst channel"))?,
            src_connection_id: connection_id.clone(),
            dst_connection_id: connection_id.clone(),
            sequence: builder
                .sequence
                .ok_or_else(|| anyhow!("missing sequence"))?,
            timeout_height: builder
                .timeout_height
                .ok_or_else(|| anyhow!("missing timeout height"))?,
            timeout_timestamp: builder
                .timeout_timestamp
                .ok_or_else(|| anyhow!("missing timeout timestamp"))?,
            data: builder.data,
            ack: builder.ack,
            kind,
        })
    }
}

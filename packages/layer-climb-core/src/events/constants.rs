// ibc events: https://github.com/cosmos/ibc-go/blob/main/docs/events/events.md

// event types
pub const EVENT_TYPE_CONTRACT_INSTANTIATE: &str = "instantiate";
pub const EVENT_TYPE_CONTRACT_STORE_CODE: &str = "store_code";
pub const EVENT_TYPE_IBC_CREATE_CLIENT: &str = "create_client";
pub const EVENT_TYPE_IBC_CONNECTION_OPEN_INIT: &str = "connection_open_init";
pub const EVENT_TYPE_IBC_CONNECTION_OPEN_TRY: &str = "connection_open_try";
pub const EVENT_TYPE_IBC_CHANNEL_OPEN_INIT: &str = "channel_open_init";
pub const EVENT_TYPE_IBC_CHANNEL_OPEN_TRY: &str = "channel_open_try";
pub const EVENT_TYPE_IBC_SEND_PACKET: &str = "send_packet";
pub const EVENT_TYPE_IBC_RECV_PACKET: &str = "recv_packet";
pub const EVENT_TYPE_IBC_ACK_PACKET: &str = "acknowledge_packet";
pub const EVENT_TYPE_IBC_TIMEOUT_PACKET: &str = "timeout_packet";
pub const EVENT_TYPE_IBC_WRITE_ACK: &str = "write_acknowledgement";

// event attribute keys
pub const EVENT_ATTR_STORE_CODE_ID: &str = "code_id";
pub const EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V1: &str = "_contract_address";
pub const EVENT_ATTR_INSTANTIATE_CONTRACT_ADDRESS_V2: &str = "contract_address";
pub const EVENT_ATTR_IBC_CONNECTION_ID: &str = "connection_id";
pub const EVENT_ATTR_IBC_CHANNEL_ID: &str = "channel_id";
pub const EVENT_ATTR_IBC_COUNTERPARTY_CHANNEL_ID: &str = "counterparty_channel_id";
pub const EVENT_ATTR_IBC_COUNTERPARTY_VERSION: &str = "counterparty_version";
pub const EVENT_ATTR_IBC_PACKET_TIMEOUT_HEIGHT: &str = "packet_timeout_height";
pub const EVENT_ATTR_IBC_PACKET_TIMEOUT_TIMESTAMP: &str = "packet_timeout_timestamp";
pub const EVENT_ATTR_IBC_PACKET_SEQUENCE: &str = "packet_sequence";
pub const EVENT_ATTR_IBC_PACKET_SRC_PORT: &str = "packet_src_port";
pub const EVENT_ATTR_IBC_PACKET_SRC_CHANNEL: &str = "packet_src_channel";
pub const EVENT_ATTR_IBC_PACKET_DST_PORT: &str = "packet_dst_port";
pub const EVENT_ATTR_IBC_PACKET_DST_CHANNEL: &str = "packet_dst_channel";
pub const EVENT_ATTR_IBC_PACKET_CHANNEL_ORDERING: &str = "packet_channel_ordering";
pub const EVENT_ATTR_IBC_PACKET_ACK_HEX: &str = "packet_ack_hex";
pub const EVENT_ATTR_IBC_PACKET_DATA_HEX: &str = "packet_data_hex";

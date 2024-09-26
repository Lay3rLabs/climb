use super::proto::*;
use anyhow::Result;

/// the typical type used for turning protobuf messages into `Any` messages
/// especially used in transactions, and needed for multi-message transactions
/// so exported in the prelude
pub fn proto_into_any<M>(msg: &M) -> Result<Any>
where
    M: Name,
{
    Any::from_msg(msg).map_err(|e| e.into())
}

pub fn proto_into_bytes<M>(msg: &M) -> Result<Vec<u8>>
where
    M: Name,
{
    Ok(proto_into_any(msg)?.value)
}

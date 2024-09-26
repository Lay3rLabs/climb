use std::sync::LazyLock;

use serde::Serialize;

use crate::prelude::*;

static EMPTY_MSG: LazyLock<serde_json::Value> =
    LazyLock::new(|| serde_json::from_str("{}").unwrap());

/// This is a helper to create contract messages from JSON-encoded strings
/// if the supplied value is None, it will encode the cosmwasm-friendly Empty type
/// this is exported in the prelude
pub fn contract_str_to_msg<'a>(s: impl Into<Option<&'a str>>) -> Result<serde_json::Value> {
    match s.into() {
        Some(s) => serde_json::from_str(s).map_err(|err| anyhow!("{}", err)),
        None => Ok(EMPTY_MSG.clone()),
    }
}

// this is an internal helper to convert a message to a vec of bytes, not part of the public API
pub fn contract_msg_to_vec(s: &impl Serialize) -> Result<Vec<u8>> {
    cosmwasm_std::to_json_vec(s).map_err(|err| anyhow!("{}", err))
}

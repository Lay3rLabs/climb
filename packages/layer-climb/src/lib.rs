pub mod prelude;

#[cfg(not(target_arch = "wasm32"))]
pub use layer_climb_core::pool;
pub use layer_climb_core::{
    // listing manually so we can excluse the prelude (which is re-exported in the prelude module here, along with config, address, etc.)
    // and not confuse ide's with multiple preludes
    contract_helpers,
    error,
    events,
    ibc_types,
    network,
    querier,
    signing,
    transaction,
};

// in case anyone wants to use the protobufs directly
pub mod proto {
    pub use layer_climb_proto::*;
}

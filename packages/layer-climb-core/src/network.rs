pub mod rpc;

use crate::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", target_os = "unknown"))] {
        pub mod grpc_web;
    } else if #[cfg(target_arch = "wasm32")] {
        pub mod grpc_wasi;
    } else {
        pub mod grpc_native;
    }
}

pub fn apply_grpc_height<T>(req: &mut tonic::Request<T>, height: Option<u64>) -> Result<()> {
    if let Some(height) = height {
        req.metadata_mut()
            .insert("x-cosmos-block-height", height.to_string().try_into()?);
    }

    Ok(())
}

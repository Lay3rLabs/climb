use anyhow::Result;
use layer_climb_config::ChainConfig;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::network::rpc::{RpcClient, RpcTransport};

/// This cache is on the QueryClient and can be used
/// to either pre-populate the cache with resources created on the outside
/// or reuse them between climb clients
///
/// however, the clients themselves hold onto their resoruces
/// so a cache is _not_ needed if you're just cloning clients around
#[derive(Clone)]
pub struct ClimbCache {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    grpc: Arc<Mutex<HashMap<String, tonic_web_wasm_client::Client>>>,
    #[cfg(all(target_arch = "wasm32", not(target_os = "unknown")))]
    #[allow(dead_code)]
    grpc: Arc<Mutex<HashMap<String, crate::network::grpc_wasi::Client>>>,
    #[cfg(not(target_arch = "wasm32"))]
    grpc: Arc<Mutex<HashMap<String, tonic::transport::Channel>>>,
    rpc: Arc<Mutex<HashMap<String, RpcClient>>>,
    rpc_transport: Arc<dyn RpcTransport>,
}

impl ClimbCache {
    pub fn new(rpc_transport: Arc<dyn RpcTransport>) -> Self {
        Self {
            grpc: Arc::new(Mutex::new(HashMap::new())),
            rpc: Arc::new(Mutex::new(HashMap::new())),
            rpc_transport,
        }
    }
}

impl ClimbCache {
    pub fn get_rpc_client(&self, config: &ChainConfig) -> Option<RpcClient> {
        match config.rpc_endpoint.as_ref() {
            None => None,
            Some(endpoint) => {
                let rpc = { self.rpc.lock().unwrap().get(endpoint).cloned() };

                Some(match rpc {
                    Some(rpc) => rpc,
                    None => {
                        let rpc = RpcClient::new(endpoint.to_string(), self.rpc_transport.clone());
                        self.rpc
                            .lock()
                            .unwrap()
                            .insert(endpoint.to_string(), rpc.clone());
                        rpc
                    }
                })
            }
        }
    }
}
cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "wasm32", target_os = "unknown"))] {
        impl ClimbCache {
            pub async fn get_web_grpc(&self, chain_config: &ChainConfig) -> Result<Option<tonic_web_wasm_client::Client>> {
                let endpoint = match chain_config.grpc_web_endpoint.as_ref() {
                    Some(endpoint) => endpoint.to_string(),
                    None => match chain_config.grpc_endpoint.as_ref() {
                        Some(endpoint) => endpoint.to_string(),
                        None => return Ok(None),
                    }
                };


                let grpc = {
                    self.grpc.lock().unwrap().get(&endpoint).cloned()
                };

                Ok(Some(match grpc {
                    Some(grpc) => grpc,
                    None => {
                        let grpc = crate::network::grpc_web::make_grpc_client(endpoint.clone()).await?;
                        self.grpc.lock().unwrap().insert(endpoint, grpc.clone());
                        grpc
                    }
                }))
            }
        }
    } else if #[cfg(target_arch = "wasm32")] {
        impl ClimbCache {
            pub async fn get_wasi_grpc(&self, _chain_config: &ChainConfig) -> Result<Option<crate::network::grpc_wasi::Client>> {
                unimplemented!();
            }
        }
    } else {
        impl ClimbCache {
            pub async fn get_grpc(&self, chain_config: &ChainConfig) -> Result<Option<tonic::transport::Channel>> {
                match chain_config.grpc_endpoint.as_ref() {
                    None => Ok(None),
                    Some(endpoint) => {
                        let grpc = {
                            self.grpc.lock().unwrap().get(endpoint).cloned()
                        };

                        Ok(Some(match grpc {
                            Some(grpc) => grpc,
                            None => {
                                tracing::debug!("Creating new grpc channel for {}", endpoint);
                                let grpc = crate::network::grpc_native::make_grpc_channel(endpoint).await?;
                                self.grpc.lock().unwrap().insert(endpoint.to_string(), grpc.clone());
                                grpc
                            }
                        }))
                    }
                }
            }
        }
    }
}

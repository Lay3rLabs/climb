use anyhow::Result;
use layer_climb_config::ChainConfig;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::network::rpc::RpcClient;

/// This cache is on the QueryClient and can be used
/// to either pre-populate the cache with resources created on the outside
/// or reuse them between climb clients
///
/// however, the clients themselves hold onto their resoruces
/// so a cache is _not_ needed if you're just cloning clients around
#[derive(Clone)]
pub struct ClimbCache {
    #[cfg(target_arch = "wasm32")]
    grpc: Arc<Mutex<HashMap<String, tonic_web_wasm_client::Client>>>,
    #[cfg(not(target_arch = "wasm32"))]
    grpc: Arc<Mutex<HashMap<String, tonic::transport::Channel>>>,
    rpc: Arc<Mutex<HashMap<String, RpcClient>>>,
    http: Arc<Mutex<Option<reqwest::Client>>>,
}

impl Default for ClimbCache {
    fn default() -> Self {
        Self {
            grpc: Arc::new(Mutex::new(HashMap::new())),
            rpc: Arc::new(Mutex::new(HashMap::new())),
            http: Arc::new(Mutex::new(None)),
        }
    }
}

impl ClimbCache {
    pub fn get_http_client(&self) -> reqwest::Client {
        let client = { self.http.lock().unwrap().clone() };

        match client {
            Some(client) => client,
            None => {
                let client = reqwest::Client::new();
                *self.http.lock().unwrap() = Some(client.clone());
                client
            }
        }
    }

    pub fn get_rpc_client(&self, config: &ChainConfig) -> Option<RpcClient> {
        match config.rpc_endpoint.as_ref() {
            None => None,
            Some(endpoint) => {
                let rpc = { self.rpc.lock().unwrap().get(endpoint).cloned() };

                Some(match rpc {
                    Some(rpc) => rpc,
                    None => {
                        let rpc = RpcClient::new(endpoint.to_string(), self.get_http_client());
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
    if #[cfg(target_arch = "wasm32")] {
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

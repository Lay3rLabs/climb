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

    pub fn get_rpc_client(&self, url: &str) -> RpcClient {
        let rpc = { self.rpc.lock().unwrap().get(url).cloned() };

        match rpc {
            Some(rpc) => rpc,
            None => {
                let rpc = RpcClient::new(url.to_string(), self.get_http_client());
                self.rpc
                    .lock()
                    .unwrap()
                    .insert(url.to_string(), rpc.clone());
                rpc
            }
        }
    }
}
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        impl ClimbCache {
            pub async fn get_grpc(&self, chain_config: &ChainConfig) -> Result<tonic_web_wasm_client::Client> {

                let endpoint = chain_config
                    .grpc_web_endpoint
                    .as_ref()
                    .unwrap_or(&chain_config.grpc_endpoint)
                    .to_string();

                let grpc = {
                    self.grpc.lock().unwrap().get(&endpoint).cloned()
                };

                Ok(match grpc {
                    Some(grpc) => grpc,
                    None => {
                        let grpc = crate::network::grpc_web::make_grpc_client(endpoint.clone()).await?;
                        self.grpc.lock().unwrap().insert(endpoint, grpc.clone());
                        grpc
                    }
                })
            }
        }
    } else {
        impl ClimbCache {
            pub async fn get_grpc(&self, chain_config: &ChainConfig) -> Result<tonic::transport::Channel> {
                let grpc = {
                    self.grpc.lock().unwrap().get(&chain_config.grpc_endpoint).cloned()
                };

                Ok(match grpc {
                    Some(grpc) => grpc,
                    None => {
                        let grpc = crate::network::grpc_native::make_grpc_channel(&chain_config.grpc_endpoint).await?;
                        self.grpc.lock().unwrap().insert(chain_config.grpc_endpoint.to_string(), grpc.clone());
                        grpc
                    }
                })
            }
        }
    }
}

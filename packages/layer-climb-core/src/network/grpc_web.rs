use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use tonic_web_wasm_client::Client;

use crate::prelude::*;

static GRPC_CLIENT_CACHE: LazyLock<GrpcClientCache> = LazyLock::new(GrpcClientCache::new);

// This struct is just an internal cache so we don't have to reconnect to the same chain multiple times
// it's intentionally *not* public so that the API is simply "get me a client in the most efficient way possible"
struct GrpcClientCache {
    clients: Mutex<HashMap<String, Client>>,
}

impl GrpcClientCache {
    fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }
}

pub async fn get_grpc_client(config: &ChainConfig) -> Result<Client> {
    // try to get the channel from the cache
    let client = {
        // give the lock its own scope so it can be definitively dropped before the await
        let lock = GRPC_CLIENT_CACHE.clients.lock().unwrap();
        lock.get(&config.grpc_endpoint).cloned()
    };

    match client {
        Some(client) => Ok(client),
        None => {
            let client = Client::new(config.grpc_endpoint.clone());

            GRPC_CLIENT_CACHE
                .clients
                .lock()
                .unwrap()
                .insert(config.grpc_endpoint.clone(), client.clone());

            Ok(client)
        }
    }
}

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

use crate::prelude::*;
use tonic::transport::{Channel, ClientTlsConfig};

static GRPC_CHANNEL_CACHE: LazyLock<GrpcChannelCache> = LazyLock::new(GrpcChannelCache::new);

// This struct is just an internal cache so we don't have to reconnect to the same chain multiple times
// it's intentionally *not* public so that the API is simply "get me a channel in the most efficient way possible"
struct GrpcChannelCache {
    channels: Mutex<HashMap<String, Channel>>,
}

impl GrpcChannelCache {
    fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }
}

pub async fn get_grpc_channel(config: &ChainConfig) -> Result<Channel> {
    // try to get the channel from the cache
    let channel = {
        // give the lock its own scope so it can be definitively dropped before the await
        let lock = GRPC_CHANNEL_CACHE.channels.lock().unwrap();
        lock.get(&config.grpc_endpoint).cloned()
    };

    match channel {
        Some(channel) => Ok(channel),
        None => {
            let endpoint_uri = config.grpc_endpoint.parse::<tonic::transport::Uri>()?;

            let channel =
                tonic::transport::Endpoint::new(endpoint_uri).map_err(|err| anyhow!("{}", err))?;

            let tls_config = ClientTlsConfig::new().with_enabled_roots();

            // see  https://jessitron.com/2022/11/02/make-https-work-on-grpc-in-rust-load-a-root-certificate-into-the-tls-config/
            // if let Ok(pem) = match std::fs::read_to_string("/etc/ssl/cert.pem") {
            //     let ca = Certificate::from_pem(pem);
            //     tls_config = tls_config.ca_certificate(ca);
            // }

            let channel = channel
                .tls_config(tls_config)?
                .connect()
                .await
                .map_err(|err| anyhow!("error connecting on {}: {}", config.grpc_endpoint, err))?;

            GRPC_CHANNEL_CACHE
                .channels
                .lock()
                .unwrap()
                .insert(config.grpc_endpoint.clone(), channel.clone());

            Ok(channel)
        }
    }
}

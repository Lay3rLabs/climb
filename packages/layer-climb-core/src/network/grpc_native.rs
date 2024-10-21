use crate::prelude::*;
use tonic::transport::{Channel, ClientTlsConfig};

pub async fn make_grpc_channel(endpoint: &str) -> Result<Channel> {
    let endpoint_uri = endpoint.parse::<tonic::transport::Uri>()?;

    let channel =
        tonic::transport::Endpoint::new(endpoint_uri).map_err(|err| anyhow!("{}", err))?;

    let tls_config = ClientTlsConfig::new().with_enabled_roots();

    // see  https://jessitron.com/2022/11/02/make-https-work-on-grpc-in-rust-load-a-root-certificate-into-the-tls-config/
    // if let Ok(pem) = match std::fs::read_to_string("/etc/ssl/cert.pem") {
    //     let ca = Certificate::from_pem(pem);
    //     tls_config = tls_config.ca_certificate(ca);
    // }

    channel
        .tls_config(tls_config)?
        .connect()
        .await
        .map_err(|err| anyhow!("error connecting on {}: {}", endpoint, err))
}

use tonic_web_wasm_client::Client;

use crate::prelude::*;

pub async fn make_grpc_client(endpoint: String) -> Result<Client> {
    Ok(Client::new(endpoint))
}

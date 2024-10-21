// see this example for testing axum: https://github.com/tokio-rs/axum/tree/main/examples/testing

use std::sync::{LazyLock, OnceLock};

use axum::{
    body::Body,
    http::{Method, Request},
    Router,
};
use bip39::Mnemonic;
use http_body_util::BodyExt;
use layer_climb::prelude::*;
use layer_climb_faucet::{
    config::{Config, ConfigInit},
    handlers::{
        credit::{CreditRequest, CreditResponse},
        status::StatusResponse,
    },
};
use rand::{prelude::*, rngs::OsRng};
use serde::de::DeserializeOwned;
use tower::{Service, ServiceExt};
use tracing::subscriber::DefaultGuard;

pub struct App {
    pub config: Config,
    pub rng: OsRng,
    pub router: Router,
    _tracing_guard: DefaultGuard,
}

// need a static reference to the app to avoid double initialization
// otherwise we defeat the purpose of a pool
// if we change the test suite to use reqwest instead of direct axum, we can remove this
// and each App will be a genuinely new instance
static ROUTER: OnceLock<Router> = OnceLock::new();
// just so we don't reload it each time
static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::try_from(ConfigInit::load_sync("./config/faucet-layer-test.toml").unwrap()).unwrap()
});

impl App {
    pub async fn new() -> Self {
        let config = CONFIG.clone();

        if ROUTER.get().is_none() {
            let router = layer_climb_faucet::router::make_router(config.clone())
                .await
                .unwrap();
            let _ = ROUTER.set(router.clone());
        };

        let router = ROUTER.get().unwrap().clone();

        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_target(false)
            .with_max_level(config.tracing_level)
            .finish();

        // Set the subscriber for this scope
        let _tracing_guard = tracing::subscriber::set_default(subscriber);

        let rng = OsRng;

        Self {
            config,
            rng,
            router,
            _tracing_guard,
        }
    }

    // get an instance of the router, but wait for it to be ready
    async fn router(&mut self) -> &mut Router {
        <Router as tower::ServiceExt<Request<Body>>>::ready(&mut self.router)
            .await
            .unwrap();
        &mut self.router
    }

    pub async fn status(&mut self) -> StatusResponse {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/status")
            .body(Body::empty())
            .unwrap();

        let response = self.router().await.oneshot(req).await.unwrap();

        map_response(response).await
    }

    pub async fn credit(&mut self, data: CreditRequest) -> CreditResponse {
        let body = serde_json::to_string(&data).unwrap();
        let req = Request::builder()
            .method(Method::POST)
            .header("Content-Type", "application/json")
            .uri("/credit")
            .body(body)
            .unwrap();

        let response = self.router().await.call(req).await.unwrap();

        map_response(response).await
    }

    pub async fn generate_signing_client(&mut self) -> SigningClient {
        let entropy: [u8; 32] = self.rng.gen();
        let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();

        let signer = KeySigner::new_mnemonic_iter(mnemonic.word_iter(), None).unwrap();

        SigningClient::new(self.config.chain_config.clone(), signer)
            .await
            .unwrap()
    }
}

#[allow(dead_code)]
async fn assert_empty_response(response: axum::http::Response<Body>) {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();

    //println!("the bytes: {}", std::str::from_utf8(&bytes).unwrap());

    assert!(bytes.is_empty());
}

async fn map_response<T: DeserializeOwned>(response: axum::http::Response<Body>) -> T {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

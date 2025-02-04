// see this example for testing axum: https://github.com/tokio-rs/axum/tree/main/examples/testing

use std::{
    ops::Mul,
    sync::{LazyLock, OnceLock},
};

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
        status::{StatusCoin, StatusResponse},
    },
};
use rand::{rngs::OsRng, TryRngCore};
use serde::de::DeserializeOwned;
use tower::Service;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// // need a static reference to the app to avoid double initialization
// static ROUTER: LazyLock<Mutex<Option<Router>>> = LazyLock::new(|| Mutex::new(None));

// just so we don't reload it each time
pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    Config::try_from(ConfigInit::load_sync("./config/faucet-layer-test.toml").unwrap()).unwrap()
});

static INIT: LazyLock<tokio::sync::Mutex<bool>> = LazyLock::new(|| tokio::sync::Mutex::new(false));

static FAUCET: LazyLock<tokio::sync::Mutex<Option<SigningClient>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(None));
static ORIGINAL_FAUCET: OnceLock<SigningClient> = OnceLock::new();

// we fund the per-test unique faucet by some amount that should cover all test cases
// right now it's the max of the topup and credit, times 1000... why not
pub static FAUCET_FUND_AMOUNT: LazyLock<u128> = LazyLock::new(|| {
    CONFIG
        .credit
        .amount
        .parse::<u128>()
        .unwrap()
        .max(CONFIG.minimum_credit_balance_topup)
        .mul(1000)
});

//static CACHE: LazyLock<ClimbCache> = LazyLock::new(|| ClimbCache::default());

#[derive(Clone)]
pub struct App {
    _router: Router,
    pub query_client: QueryClient,
    pub config: Config,
}

// this is called from every test, but we gate it with an async-aware lock so it only runs once
async fn init() -> Result<(), String> {
    let mut lock = INIT.lock().await;

    if !*lock {
        *lock = true;
        let mut tracing_env = tracing_subscriber::EnvFilter::from_default_env();
        for directive in &CONFIG.tracing_directives {
            tracing_env = tracing_env.add_directive(directive.parse().unwrap());
        }

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .without_time()
                    .with_target(false),
            )
            .with(tracing_env)
            .try_init()
            .unwrap();
    }

    let original_faucet_signer =
        KeySigner::new_mnemonic_str(&CONFIG.mnemonic.clone(), None).unwrap();

    match SigningClient::new(CONFIG.chain_config.clone(), original_faucet_signer, None).await {
        Ok(original_faucet) => {
            ORIGINAL_FAUCET.set(original_faucet).unwrap();
            Ok(())
        }
        Err(e) => Err(format!("Failed to create original faucet: {}", e)),
    }
}

// this is called from every test, but we gate it with an async-aware lock so there's no sequence errors
async fn fund_faucet(addr: &Address) {
    let mut lock = FAUCET.lock().await;
    if lock.is_none() {
        let faucet_signer = KeySigner::new_mnemonic_str(&CONFIG.mnemonic.clone(), None).unwrap();
        let faucet = SigningClient::new(CONFIG.chain_config.clone(), faucet_signer, None)
            .await
            .unwrap();

        *lock = Some(faucet);
    }

    tracing::debug!(
        "FUNDING PER-ROUTER FAUCET ({} from {} to {})",
        *FAUCET_FUND_AMOUNT,
        lock.as_ref().unwrap().addr,
        addr
    );

    lock.as_ref()
        .unwrap()
        .transfer(
            *FAUCET_FUND_AMOUNT,
            addr,
            Some(CONFIG.credit.denom.as_str()),
            None,
        )
        .await
        .unwrap();
}

impl App {
    pub async fn new() -> Option<Self> {
        if let Err(e) = init().await {
            tracing::warn!("skipping faucet test: {}", e);
            return None;
        }

        // generate a new faucet per application - otherwise they will have sequence errors
        // because each test is run in a different thread
        let faucet_mnemonic = generate_mnemonic();
        let faucet_addr = CONFIG
            .chain_config
            .address_from_pub_key(
                &KeySigner::new_mnemonic_iter(faucet_mnemonic.words(), None)
                    .unwrap()
                    .public_key()
                    .await
                    .unwrap(),
            )
            .unwrap();

        // replace it in the config
        let mut config = CONFIG.clone();
        config.mnemonic = faucet_mnemonic.to_string();

        // fund it
        fund_faucet(&faucet_addr).await;

        // get the router
        let router = layer_climb_faucet::router::make_router(config.clone())
            .await
            .unwrap();

        // and we're off!
        Some(Self {
            _router: router,
            query_client: QueryClient::new(config.chain_config.clone(), None)
                .await
                .unwrap(),
            config,
        })
    }

    async fn router(&mut self) -> &mut Router {
        // wait till it's ready
        <Router as tower::ServiceExt<Request<Body>>>::ready(&mut self._router)
            .await
            .unwrap();

        &mut self._router
    }

    pub async fn status(&mut self) -> StatusResponse {
        let req = Request::builder()
            .method(Method::GET)
            .uri("/status")
            .body(Body::empty())
            .unwrap();

        let response = self.router().await.call(req).await.unwrap();

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
}

fn generate_mnemonic() -> Mnemonic {
    let mut entropy = [0u8; 32];
    OsRng.try_fill_bytes(&mut entropy).unwrap();
    Mnemonic::from_entropy(&entropy).unwrap()
}

pub async fn generate_signing_client() -> SigningClient {
    let signer = KeySigner::new_mnemonic_iter(generate_mnemonic().words(), None).unwrap();

    SigningClient::new(CONFIG.chain_config.clone(), signer, None)
        .await
        .unwrap()
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

pub fn extract_status_balance(balances: &[StatusCoin]) -> u128 {
    balances
        .iter()
        .find(|b| b.denom == CONFIG.credit.denom)
        .map(|b| b.amount.parse::<u128>().unwrap())
        .unwrap_or(0)
}

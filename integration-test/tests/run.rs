use layer_climb::prelude::*;
use layer_climb_cli::handle::CosmosInstance;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::test]
async fn run() {
    // https://github.com/rustls/rustls/issues/1938#issuecomment-2567934864
    let _ = rustls::crypto::ring::default_provider().install_default();

    if dotenvy::dotenv().is_err() {
        eprintln!("Warning: no .env file found, did you copy .env.example over?");
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .with_target(false),
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .try_init()
        .unwrap();

    let chain_config = ChainConfig {
        chain_id: "wasmd-test".parse().unwrap(),
        rpc_endpoint: Some("http://127.0.0.1:26657".to_string()),
        grpc_endpoint: Some("http://127.0.0.1:9090".to_string()),
        grpc_web_endpoint: None,
        gas_price: 0.025,
        gas_denom: "ucosm".to_string(),
        address_kind: AddrKind::Cosmos {
            prefix: "wasm".to_string(),
        },
    };

    let mnemonic = std::env::var("CLIMB_TEST_MNEMONIC")
        .expect("Missing 'CLIMB_TEST_MNEMONIC' in environment.");
    let signer_1 = KeySigner::new_mnemonic_str(&mnemonic, None).unwrap();
    let signer_2 =
        KeySigner::new_mnemonic_str(&mnemonic, Some(&cosmos_hub_derivation(2).unwrap())).unwrap();
    let signer_3 =
        KeySigner::new_mnemonic_str(&mnemonic, Some(&cosmos_hub_derivation(3).unwrap())).unwrap();
    let signer_addr_1 = chain_config
        .address_from_pub_key(&signer_1.public_key().await.unwrap())
        .unwrap();
    let signer_addr_2 = chain_config
        .address_from_pub_key(&signer_2.public_key().await.unwrap())
        .unwrap();
    let signer_addr_3 = chain_config
        .address_from_pub_key(&signer_3.public_key().await.unwrap())
        .unwrap();

    let cosmos_instance = CosmosInstance::new(
        chain_config.clone(),
        vec![signer_addr_1.clone(), signer_addr_2.clone()],
    );
    // cosmos_instance.stderr = StdioKind::Inherit;
    // cosmos_instance.stdout = StdioKind::Inherit;
    cosmos_instance.start().await.unwrap();

    let client = SigningClient::new(chain_config, signer_1, None)
        .await
        .unwrap();

    // check that our original status is
    // client 1: has some balance
    // client 2: has some balance
    // client 3: has no balance
    let original_balance = client
        .querier
        .balance(signer_addr_1.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();
    assert!(original_balance > 0);

    let balance = client
        .querier
        .balance(signer_addr_2.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();
    assert!(balance > 0);

    let balance = client
        .querier
        .balance(signer_addr_3.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();
    assert_eq!(balance, 0);

    // transfer from client 1 to client 3 and check:
    // client 1: has less balance than originally (we're using same denom as gas, so we aren't checking exact amounts)
    // client 3: has exactly 100
    client
        .transfer(100, &signer_addr_3, None, None)
        .await
        .unwrap();

    let balance = client
        .querier
        .balance(signer_addr_1, None)
        .await
        .unwrap()
        .unwrap_or_default();
    assert!(balance < original_balance);

    let balance = client
        .querier
        .balance(signer_addr_3, None)
        .await
        .unwrap()
        .unwrap_or_default();
    assert_eq!(balance, 100);
}

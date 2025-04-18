mod helpers;
use helpers::{extract_status_balance, generate_signing_client, App, CONFIG, FAUCET_FUND_AMOUNT};
use layer_climb::prelude::*;
use layer_climb_faucet::handlers::credit::CreditRequest;

#[tokio::test]
async fn status_ok() {
    // https://github.com/rustls/rustls/issues/1938#issuecomment-2567934864
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut app = match App::new().await {
        Some(app) => app,
        None => return,
    };

    let status = app.status().await;

    let faucet_addr = CONFIG
        .chain_config
        .address_from_pub_key(
            &KeySigner::new_mnemonic_str(&CONFIG.mnemonic, None)
                .unwrap()
                .public_key()
                .await
                .unwrap(),
        )
        .unwrap();

    assert!(
        status.holder.address != faucet_addr.to_string(),
        "Expected holder address to be different from faucet address."
    );
    assert_eq!(
        extract_status_balance(&status.holder.balances),
        *FAUCET_FUND_AMOUNT,
        "Expected holder balance to be equal to faucet fund amount."
    );
}

#[tokio::test]
async fn credit_works() {
    // https://github.com/rustls/rustls/issues/1938#issuecomment-2567934864
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut app = match App::new().await {
        Some(app) => app,
        None => return,
    };

    let client = generate_signing_client().await;

    let balance_before = app
        .query_client
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    app.credit(CreditRequest {
        address: client.addr.to_string(),
        denom: None,
    })
    .await;

    let balance_after = app
        .query_client
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    assert!(
        balance_after > balance_before,
        "Expected for {} balance_after ({}) to be greater than balance_before ({}).",
        client.addr,
        balance_after,
        balance_before
    );
}

#[tokio::test]
async fn credit_works_multi_distribution_serial() {
    // https://github.com/rustls/rustls/issues/1938#issuecomment-2567934864
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut app = match App::new().await {
        Some(app) => app,
        None => return,
    };

    let client = generate_signing_client().await;

    let balance_before = app
        .query_client
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    app.credit(CreditRequest {
        address: client.addr.to_string(),
        denom: None,
    })
    .await;

    app.credit(CreditRequest {
        address: client.addr.to_string(),
        denom: None,
    })
    .await;

    app.credit(CreditRequest {
        address: client.addr.to_string(),
        denom: None,
    })
    .await;

    let balance_after = app
        .query_client
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    let expected_balance = balance_before + app.config.credit.amount.parse::<u128>().unwrap() * 3;

    assert!(
        balance_after >= expected_balance,
        "Expected for {} balance_after ({}) to be greater than or equal to expected balance ({}), but it is {}.",
        client.addr, balance_after, expected_balance, balance_after
    );

    let status = app.status().await;

    assert!(
        status.distributors.len() == 1,
        "Expected 1 distributor, got {:?}",
        status.distributors
    );

    let holder_balance = extract_status_balance(&status.holder.balances);
    assert!(
        holder_balance < *FAUCET_FUND_AMOUNT,
        "Expected holder balance ({}) to be less than faucet fund amount ({}).",
        holder_balance,
        *FAUCET_FUND_AMOUNT
    );
}

#[tokio::test]
async fn credit_works_multi_distribution_concurrent() {
    // https://github.com/rustls/rustls/issues/1938#issuecomment-2567934864
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut app = match App::new().await {
        Some(app) => app,
        None => return,
    };

    let client = generate_signing_client().await;

    let balance_before = app
        .query_client
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    tokio::join!(
        {
            let mut app = app.clone();
            let client = client.clone();
            async move {
                app.credit(CreditRequest {
                    address: client.addr.to_string(),
                    denom: None,
                })
                .await;
            }
        },
        {
            let mut app = app.clone();
            let client = client.clone();
            async move {
                app.credit(CreditRequest {
                    address: client.addr.to_string(),
                    denom: None,
                })
                .await;
            }
        },
        {
            let mut app = app.clone();
            let client = client.clone();
            async move {
                app.credit(CreditRequest {
                    address: client.addr.to_string(),
                    denom: None,
                })
                .await;
            }
        },
    );

    let balance_after = app
        .query_client
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    let expected_balance = balance_before + app.config.credit.amount.parse::<u128>().unwrap() * 3;

    assert!(
        balance_after >= expected_balance,
        "Expected for {} balance_after ({}) to be greater than or equal to expected balance ({}), but it is {}.",
        client.addr, balance_after, expected_balance, balance_after
    );

    let status = app.status().await;

    assert!(
        status.distributors.len() == 3,
        "Expected 3 distributors, got {:?}",
        status.distributors
    );

    let holder_balance = extract_status_balance(&status.holder.balances);
    assert!(
        holder_balance < *FAUCET_FUND_AMOUNT,
        "Expected holder balance ({}) to be less than faucet fund amount ({}).",
        holder_balance,
        *FAUCET_FUND_AMOUNT
    );
}

#[tokio::test]
async fn send_to_self_works() {
    // https://github.com/rustls/rustls/issues/1938#issuecomment-2567934864
    let _ = rustls::crypto::ring::default_provider().install_default();

    async fn run_test(mut app: App, dest_addr: Address) {
        let balance_before = app
            .query_client
            .balance(dest_addr.clone(), None)
            .await
            .unwrap()
            .unwrap_or_default();

        app.credit(CreditRequest {
            address: dest_addr.to_string(),
            denom: None,
        })
        .await;

        let balance_after = app
            .query_client
            .balance(dest_addr.clone(), None)
            .await
            .unwrap()
            .unwrap_or_default();

        assert!(
            balance_after != balance_before,
            "Expected for {} balance_after ({}) to be different than balance_before ({}).",
            dest_addr,
            balance_after,
            balance_before
        );
    }

    let mut app = match App::new().await {
        Some(app) => app,
        None => return,
    };

    // force at least one distributor
    let dummy = generate_signing_client().await;
    app.credit(CreditRequest {
        address: dummy.addr.to_string(),
        denom: None,
    })
    .await;

    let status = app.status().await;

    assert!(
        !status.distributors.is_empty(),
        "Expected at least one distributor."
    );

    let mut addrs = status
        .distributors
        .iter()
        .map(|distributor| {
            app.config
                .chain_config
                .parse_address(&distributor.address)
                .unwrap()
        })
        .collect::<Vec<_>>();

    addrs.push(
        app.config
            .chain_config
            .parse_address(&status.holder.address)
            .unwrap(),
    );

    for addr in addrs {
        run_test(app.clone(), addr).await;
    }
}

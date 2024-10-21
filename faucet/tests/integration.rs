mod helpers;
use helpers::App;
use layer_climb_faucet::handlers::credit::CreditRequest;

#[tokio::test]
async fn status_ok() {
    let mut app = App::new().await;

    let _ = app.status().await;
}

#[tokio::test]
async fn credit_works() {
    let mut app = App::new().await;

    let client = app.generate_signing_client().await;

    let balance_before = client
        .querier
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    app.credit(CreditRequest {
        address: client.addr.to_string(),
        denom: None,
    })
    .await;

    let balance_after = client
        .querier
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
    let mut app = App::new().await;

    let client = app.generate_signing_client().await;

    let balance_before = client
        .querier
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

    let balance_after = client
        .querier
        .balance(client.addr.clone(), None)
        .await
        .unwrap()
        .unwrap_or_default();

    let expected_balance = balance_before + app.config.credit.amount.parse::<u128>().unwrap() * 3;

    assert!(
        balance_after > expected_balance,
        "Expected for {} balance_after ({}) to be greater than expected balance ({}), but it is {}.",
        client.addr, balance_after, expected_balance, balance_after
    );
}

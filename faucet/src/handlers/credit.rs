use crate::prelude::*;
use axum::extract;

#[derive(Deserialize, Debug)]
pub struct CreditRequest {
    pub address: String,
    pub denom: Option<String>,
}

#[axum::debug_handler]
pub async fn credit(
    State(state): State<AppState>,
    extract::Json(payload): extract::Json<CreditRequest>,
) -> Result<()> {
    tracing::debug!("credit request: {:?}", payload);

    let address = state.config.chain_config.parse_address(&payload.address)?;

    if let Some(denom) = &payload.denom {
        if denom != state.config.credit.denom.as_str() {
            return Err(anyhow::anyhow!("invalid denom").into());
        }
    }

    // do not send to ourselves
    let sender = loop {
        let sender = state
            .client_pool
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("{e:?}"))?;
        if sender.addr != address {
            break sender;
        }
    };

    let mut tx_builder = sender.tx_builder();
    if let Some(memo) = &state.config.memo {
        tx_builder.set_memo(memo);
    }

    let tx = sender
        .transfer(
            state.config.credit.amount.parse()?,
            &address,
            Some(state.config.credit.denom.as_str()),
            Some(tx_builder),
        )
        .await?;

    tracing::debug!("sent credit to {}, tx hash: {}", address, tx.txhash);

    Ok(())
}

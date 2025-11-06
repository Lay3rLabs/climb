use crate::prelude::*;
use axum::{extract, Json};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreditRequest {
    pub address: String,
    pub denom: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CreditResponse {
    pub amount: u128,
    pub recipient: Address,
    pub denom: String,
    pub txhash: String,
}

#[axum::debug_handler]
pub async fn credit(
    State(state): State<AppState>,
    extract::Json(payload): extract::Json<CreditRequest>,
) -> impl IntoResponse {
    match credit_inner(state, payload).await {
        Ok(resp) => Json(resp).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn credit_inner(state: AppState, payload: CreditRequest) -> Result<CreditResponse> {
    tracing::debug!("credit request: {:?}", payload);

    let address = state.config.chain_config.parse_address(&payload.address)?;

    if let Some(denom) = &payload.denom {
        if denom != state.config.credit.denom.as_str() {
            return Err(ClimbError::from(ValidationError::invalid_denom(
                &state.config.credit.denom,
                denom,
            ))
            .into());
        }
    }

    let sender = state
        .client_pool
        .get()
        .await
        .map_err(|e| ClimbError::from(NetworkError::client_pool(format!("{e:?}"))))?;

    let mut tx_builder = sender.tx_builder();
    if let Some(memo) = &state.config.memo {
        tx_builder.set_memo(memo);
    }

    let amount = state
        .config
        .credit
        .amount
        .parse()
        .map_err(|e| ClimbError::parse(format!("{e}")))?;

    let tx = sender
        .transfer(
            amount,
            &address,
            Some(state.config.credit.denom.as_str()),
            Some(tx_builder),
        )
        .await?;

    Ok(CreditResponse {
        amount,
        recipient: address,
        denom: state.config.credit.denom.clone(),
        txhash: tx.txhash,
    })
}

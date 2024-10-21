pub mod contract;
pub mod ibc;
pub mod middleware;
pub mod msg;

use crate::{
    cache::ClimbCache, prelude::*, transaction::{SequenceStrategy, SequenceStrategyKind}
};
use layer_climb_address::TxSigner;
use middleware::{SigningMiddlewareMapBody, SigningMiddlewareMapResp};
use tracing::instrument;
use std::sync::Arc;

// Cloning a SigningClient is pretty cheap
#[derive(Clone)]
pub struct SigningClient {
    pub querier: QueryClient,
    pub signer: Arc<dyn TxSigner>,
    pub addr: Address,
    pub account_number: u64,
    /// Middleware to run before the tx is broadcast
    pub middleware_map_body: Arc<Vec<SigningMiddlewareMapBody>>,
    /// Middleware to run after the tx is broadcast
    pub middleware_map_resp: Arc<Vec<SigningMiddlewareMapResp>>,
    /// Strategy for determining the sequence number for txs
    /// it will be applied when calling `tx_builder()`
    /// (i.e. it's always possible to manually construct a TxBuilder and override it)
    /// Default is `SequenceStrategyKind::Query`
    pub sequence_strategy: SequenceStrategy,
}

impl std::fmt::Debug for SigningClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningClient")
            .field("chain_id", &self.querier.chain_config.chain_id)
            .field("addr", &self.addr)
            .finish()
    }
}

impl SigningClient {
    pub async fn new(chain_config: ChainConfig, signer: impl TxSigner + 'static) -> Result<Self> {
        Self::new_with_cache(chain_config, signer, ClimbCache::default()).await
    }

    pub async fn new_with_cache(chain_config: ChainConfig, signer: impl TxSigner + 'static, cache: ClimbCache) -> Result<Self> {
        let addr = chain_config.address_from_pub_key(&signer.public_key().await?)?;

        let querier = QueryClient::new_with_cache(chain_config.clone(), cache).await?;

        let base_account = querier.base_account(&addr).await?;

        Ok(Self {
            signer: Arc::new(signer),
            querier,
            addr,
            account_number: base_account.account_number,
            middleware_map_body: Arc::new(middleware::SigningMiddlewareMapBody::default_list()),
            middleware_map_resp: Arc::new(middleware::SigningMiddlewareMapResp::default_list()),
            sequence_strategy: SequenceStrategy::new(SequenceStrategyKind::Query),
        })
    }

    // This is especially useful if the signer gets its public key at runtime
    // such as when using browser-based async wallets like Keplr
    #[instrument]
    pub async fn refresh_signer(&mut self) -> Result<()> {
        self.addr = self
            .querier
            .chain_config
            .address_from_pub_key(&self.signer.public_key().await?)?;

        self.account_number = self.querier.base_account(&self.addr).await?.account_number;

        Ok(())
    }

    pub fn chain_id(&self) -> &ChainId {
        &self.querier.chain_config.chain_id
    }

    pub fn sequence_strategy_kind(&self) -> &SequenceStrategyKind {
        &self.sequence_strategy.kind
    }

    pub fn tx_builder(&self) -> TxBuilder<'_> {
        let mut tx_builder = TxBuilder::new(&self.querier, self.signer.as_ref());

        tx_builder
            .set_sender(self.addr.clone())
            .set_account_number(self.account_number)
            .set_sequence_strategy(self.sequence_strategy.clone());

        if self.middleware_map_body.len() > 0 {
            tx_builder.set_middleware_map_body(self.middleware_map_body.clone());
        }

        if self.middleware_map_resp.len() > 0 {
            tx_builder.set_middleware_map_resp(self.middleware_map_resp.clone());
        }

        tx_builder
    }

    pub async fn transfer<'a, D: Into<Option<&'a str>> + std::fmt::Debug>(
        &self,
        amount: u128,
        recipient: &Address,
        denom: D,
        tx_builder: Option<TxBuilder<'_>>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        tracing::debug!("transfering {} to {} from {}", amount, recipient, self.addr);

        let res = tx_builder
            .unwrap_or_else(|| self.tx_builder())
            .broadcast([proto_into_any(
                &self.transfer_msg(amount, recipient, denom)?,
            )?])
            .await?;

        Ok(res)
    }
}

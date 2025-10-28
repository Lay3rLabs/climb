use crate::prelude::*;
use crate::querier::tx::AnyTxResponse;
use crate::signing::middleware::{SigningMiddlewareMapBody, SigningMiddlewareMapResp};
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};

use layer_climb_signer::TxSigner;

pub struct TxBuilder<'a> {
    pub querier: &'a QueryClient,
    pub signer: &'a dyn TxSigner,

    /// Must be set if not providing a `sequence` or `account_number`
    pub sender: Option<Address>,

    /// how many blocks until a tx is considered invalid
    /// if not set, the default is 10 blocks
    pub tx_timeout_blocks: Option<u64>,
    /// for manually overriding the sequence number, e.g. parallel transactions (multiple *messages* in a tx do not need this)
    pub sequence_strategy: Option<SequenceStrategy>,

    /// The account number of the sender. If not set, it will be derived from the sender's account
    pub account_number: Option<u64>,

    pub memo: Option<String>,

    /// The gas coin to use. Gas price (in gas_coin.denom) = gas_coin.amount * gas_units
    /// If not set, it will be derived from querier.chain_config (without hitting the network)
    pub gas_coin: Option<layer_climb_proto::Coin>,

    /// The maximum gas units. Gas price (in gas_coin.denom) = gas_coin.amount * gas_units
    /// If not set, it will be derived from running an on-chain simulation multiplied by `gas_multiplier`
    pub gas_units_or_simulate: Option<u64>,

    /// A multiplier to use for simulated gas units.
    /// If not set, a default of 1.5 will be used.
    pub gas_simulate_multiplier: Option<f32>,

    /// The broadcast mode to use. If not set, the default is `Sync`
    pub broadcast_mode: Option<layer_climb_proto::tx::BroadcastMode>,

    /// Whether broadcasting should poll for the tx landing on chain before returning
    /// default is true
    pub broadcast_poll: bool,

    /// The duration to sleep between polling for the tx landing on chain
    /// If not set, the default is 1 second
    pub broadcast_poll_sleep_duration: Option<std::time::Duration>,

    /// The duration to wait before giving up on polling for the tx landing on chain
    /// If not set, the default is 30 seconds
    pub broadcast_poll_timeout_duration: Option<std::time::Duration>,

    /// Middleware to run before the tx is broadcast
    pub middleware_map_body: Option<Arc<Vec<SigningMiddlewareMapBody>>>,

    /// Middleware to run after the tx is broadcast
    pub middleware_map_resp: Option<Arc<Vec<SigningMiddlewareMapResp>>>,
}

impl<'a> TxBuilder<'a> {
    const DEFAULT_TX_TIMEOUT_BLOCKS: u64 = 10;
    const DEFAULT_GAS_MULTIPLIER: f32 = 1.5;
    const DEFAULT_BROADCAST_MODE: layer_climb_proto::tx::BroadcastMode =
        layer_climb_proto::tx::BroadcastMode::Sync;
    const DEFAULT_BROADCAST_POLL_SLEEP_DURATION: std::time::Duration =
        std::time::Duration::from_secs(1);
    const DEFAULT_BROADCAST_POLL_TIMEOUT_DURATION: std::time::Duration =
        std::time::Duration::from_secs(30);

    pub fn new(querier: &'a QueryClient, signer: &'a dyn TxSigner) -> Self {
        Self {
            querier,
            signer,
            gas_coin: None,
            sender: None,
            memo: None,
            tx_timeout_blocks: None,
            sequence_strategy: None,
            gas_units_or_simulate: None,
            gas_simulate_multiplier: None,
            account_number: None,
            broadcast_mode: None,
            broadcast_poll: true,
            broadcast_poll_sleep_duration: None,
            broadcast_poll_timeout_duration: None,
            middleware_map_body: None,
            middleware_map_resp: None,
        }
    }

    pub fn set_tx_timeout_blocks(&mut self, tx_timeout_blocks: u64) -> &mut Self {
        self.tx_timeout_blocks = Some(tx_timeout_blocks);
        self
    }

    pub fn set_memo(&mut self, memo: impl Into<String>) -> &mut Self {
        self.memo = Some(memo.into());
        self
    }

    pub fn set_sequence_strategy(&mut self, sequence_strategy: SequenceStrategy) -> &mut Self {
        self.sequence_strategy = Some(sequence_strategy);
        self
    }

    pub fn set_sender(&mut self, sender: Address) -> &mut Self {
        self.sender = Some(sender);
        self
    }

    pub fn set_gas_coin(&mut self, gas_coin: layer_climb_proto::Coin) -> &mut Self {
        self.gas_coin = Some(gas_coin);
        self
    }

    pub fn set_gas_units_or_simulate(&mut self, gas_units: Option<u64>) -> &mut Self {
        self.gas_units_or_simulate = gas_units;
        self
    }

    pub fn set_gas_simulate_multiplier(&mut self, gas_multiplier: f32) -> &mut Self {
        self.gas_simulate_multiplier = Some(gas_multiplier);
        self
    }

    pub fn set_account_number(&mut self, account_number: u64) -> &mut Self {
        self.account_number = Some(account_number);
        self
    }

    pub fn set_broadcast_mode(
        &mut self,
        broadcast_mode: layer_climb_proto::tx::BroadcastMode,
    ) -> &mut Self {
        self.broadcast_mode = Some(broadcast_mode);
        self
    }

    pub fn set_broadcast_poll(&mut self, broadcast_poll: bool) -> &mut Self {
        self.broadcast_poll = broadcast_poll;
        self
    }

    pub fn set_broadcast_poll_sleep_duration(
        &mut self,
        broadcast_poll_sleep_duration: std::time::Duration,
    ) -> &mut Self {
        self.broadcast_poll_sleep_duration = Some(broadcast_poll_sleep_duration);
        self
    }

    pub fn set_broadcast_poll_timeout_duration(
        &mut self,
        broadcast_poll_timeout_duration: std::time::Duration,
    ) -> &mut Self {
        self.broadcast_poll_timeout_duration = Some(broadcast_poll_timeout_duration);
        self
    }

    pub fn set_middleware_map_body(
        &mut self,
        middleware_map_body: Arc<Vec<SigningMiddlewareMapBody>>,
    ) -> &mut Self {
        self.middleware_map_body = Some(middleware_map_body);
        self
    }

    pub fn set_middleware_map_resp(
        &mut self,
        middleware_map_resp: Arc<Vec<SigningMiddlewareMapResp>>,
    ) -> &mut Self {
        self.middleware_map_resp = Some(middleware_map_resp);
        self
    }

    async fn query_base_account(&self) -> Result<layer_climb_proto::auth::BaseAccount> {
        self.querier
            .base_account(
                self.sender
                    .as_ref()
                    .with_context(|| "must provide a sender if no sequence")?,
            )
            .await
    }

    pub async fn broadcast(
        self,
        messages: impl IntoIterator<Item = layer_climb_proto::Any>,
    ) -> Result<layer_climb_proto::abci::TxResponse> {
        let messages = messages.into_iter().collect();
        let resp = self.broadcast_raw(messages).await?;

        match resp {
            AnyTxResponse::Abci(tx_response) => Ok(tx_response),
            AnyTxResponse::Rpc(_) => Err(anyhow!(
                "Unexpected AnyTxResponse type - did you mean to call broadcast_raw instead?"
            )),
        }
    }

    pub async fn simulate_gas(
        &self,
        signer_info: layer_climb_proto::tx::SignerInfo,
        account_number: u64,
        // mutable so we can set the timeout_height here
        tx_body: &mut layer_climb_proto::tx::TxBody,
    ) -> Result<layer_climb_proto::abci::GasInfo> {
        let fee = FeeCalculation::Simulation {
            chain_config: &self.querier.chain_config,
        }
        .calculate()?;

        let simulate_tx_resp = self
            .querier
            .simulate_tx(
                self.sign_tx(signer_info, account_number, tx_body, fee, true)
                    .await?,
            )
            .await?;

        simulate_tx_resp
            .gas_info
            .context("unable to get gas from simulation")
    }

    pub async fn current_sequence(&self) -> Result<u64> {
        let sequence = match &self.sequence_strategy {
            Some(sequence_strategy) => match sequence_strategy.kind {
                SequenceStrategyKind::Query => {
                    let base_account = self.query_base_account().await?;
                    base_account.sequence
                }
                SequenceStrategyKind::QueryAndIncrement => {
                    if !sequence_strategy
                        .has_queried
                        .load(std::sync::atomic::Ordering::SeqCst)
                    {
                        let base_account = self.query_base_account().await?;
                        sequence_strategy
                            .has_queried
                            .store(true, std::sync::atomic::Ordering::SeqCst);
                        sequence_strategy
                            .value
                            .store(base_account.sequence, std::sync::atomic::Ordering::SeqCst);
                        base_account.sequence
                    } else {
                        sequence_strategy
                            .value
                            .load(std::sync::atomic::Ordering::SeqCst)
                    }
                }
                SequenceStrategyKind::SetAndIncrement(_) => sequence_strategy
                    .value
                    .load(std::sync::atomic::Ordering::SeqCst),
                SequenceStrategyKind::Constant(n) => n,
            },
            None => {
                let base_account = self.query_base_account().await?;
                base_account.sequence
            }
        };

        tracing::debug!(
            "{} is Using sequence: {}",
            self.signer.address(&self.querier.chain_config).await?,
            sequence
        );

        Ok(sequence)
    }

    /// Typically do _not_ want to do this directly, use `broadcast` instead
    /// however, in a case where you do not want to wait for the tx to be committed, you can use this
    /// (and if the original tx response is AnyTxResponse::Rpc, it will stay that way)
    pub async fn broadcast_raw(
        self,
        messages: Vec<layer_climb_proto::Any>,
    ) -> Result<AnyTxResponse> {
        let account_number = match self.account_number {
            Some(account_number) => account_number,
            None => self.query_base_account().await?.account_number,
        };

        let mut body = layer_climb_proto::tx::TxBody {
            messages,
            memo: self.memo.as_deref().unwrap_or("").to_string(),
            timeout_height: 0, // will be set later so we don't get delayed by other async calls before we send
            extension_options: Default::default(),
            non_critical_extension_options: Default::default(),
        };

        if let Some(middleware) = self.middleware_map_body.as_ref() {
            for middleware in middleware.iter() {
                body = match middleware.map_body(body).await {
                    Ok(req) => req,
                    Err(e) => return Err(e),
                }
            }
        }

        let gas_units = match self.gas_units_or_simulate {
            Some(gas_units) => gas_units,
            None => {
                let gas_multiplier = self
                    .gas_simulate_multiplier
                    .unwrap_or(Self::DEFAULT_GAS_MULTIPLIER);

                let signer_info = self
                    .signer
                    .signer_info(
                        self.current_sequence().await?,
                        layer_climb_proto::tx::signing::SignMode::Unspecified,
                    )
                    .await?;

                let gas_info = self
                    .simulate_gas(signer_info, account_number, &mut body)
                    .await?;

                (gas_info.gas_used as f32 * gas_multiplier).ceil() as u64
            }
        };

        let fee = match self.gas_coin.clone() {
            Some(gas_coin) => FeeCalculation::RealCoin {
                gas_coin,
                gas_units,
            }
            .calculate()?,
            None => FeeCalculation::RealNetwork {
                chain_config: &self.querier.chain_config,
                gas_units,
            }
            .calculate()?,
        };

        let signer_info = self
            .signer
            .signer_info(
                self.current_sequence().await?,
                layer_climb_proto::tx::signing::SignMode::Direct,
            )
            .await?;

        let tx_bytes = self
            .sign_tx(signer_info, account_number, &mut body, fee, false)
            .await?;
        let broadcast_mode = self.broadcast_mode.unwrap_or(Self::DEFAULT_BROADCAST_MODE);

        let tx_response = self
            .querier
            .broadcast_tx_bytes(tx_bytes, broadcast_mode)
            .await?;

        if tx_response.code() != 0 {
            bail!(
                "tx failed with code: {}, codespace: {}, raw_log: {}",
                tx_response.code(),
                tx_response.codespace(),
                tx_response.raw_log()
            );
        }

        let mut tx_response = if self.broadcast_poll {
            let sleep_duration = self
                .broadcast_poll_sleep_duration
                .unwrap_or(Self::DEFAULT_BROADCAST_POLL_SLEEP_DURATION);
            let timeout_duration = self
                .broadcast_poll_timeout_duration
                .unwrap_or(Self::DEFAULT_BROADCAST_POLL_TIMEOUT_DURATION);

            AnyTxResponse::Abci(
                self.querier
                    .poll_until_tx_ready(tx_response.tx_hash(), sleep_duration, timeout_duration)
                    .await?
                    .tx_response,
            )
        } else {
            tx_response
        };

        if tx_response.code() != 0 {
            bail!(
                "tx failed with code: {}, codespace: {}, raw_log: {}",
                tx_response.code(),
                tx_response.codespace(),
                tx_response.raw_log()
            );
        }

        // TODO not sure about this... should increase even if failed?
        if let Some(sequence) = self.sequence_strategy {
            match sequence.kind {
                SequenceStrategyKind::QueryAndIncrement => {
                    sequence
                        .value
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                SequenceStrategyKind::SetAndIncrement(_) => {
                    sequence
                        .value
                        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                _ => {}
            }
        }

        if let Some(middleware) = self.middleware_map_resp.as_ref() {
            for middleware in middleware.iter() {
                tx_response = match middleware.map_resp(tx_response).await {
                    Ok(req) => req,
                    Err(e) => return Err(e),
                }
            }
        }

        Ok(tx_response)
    }

    async fn sign_tx(
        &self,
        signer_info: layer_climb_proto::tx::SignerInfo,
        account_number: u64,
        // mutable so we can set the timeout_height here
        body: &mut layer_climb_proto::tx::TxBody,
        fee: layer_climb_proto::tx::Fee,
        simulate_only: bool,
    ) -> Result<Vec<u8>> {
        #[allow(deprecated)]
        let auth_info = layer_climb_proto::tx::AuthInfo {
            signer_infos: vec![signer_info],
            fee: Some(fee),
            tip: None,
        };

        let block_height = self.querier.block_height().await?;

        let tx_timeout_blocks = self
            .tx_timeout_blocks
            .unwrap_or(Self::DEFAULT_TX_TIMEOUT_BLOCKS);

        // latest possible time we can grab the current block height
        body.timeout_height = block_height + tx_timeout_blocks;

        let sign_doc = layer_climb_proto::tx::SignDoc {
            body_bytes: proto_into_bytes(body)?,
            auth_info_bytes: proto_into_bytes(&auth_info)?,
            chain_id: self.querier.chain_config.chain_id.to_string(),
            account_number,
        };

        let signature = match simulate_only {
            true => Vec::new(),
            false => self.signer.sign(&sign_doc).await?,
        };

        let tx_raw = layer_climb_proto::tx::TxRaw {
            body_bytes: sign_doc.body_bytes.clone(),
            auth_info_bytes: sign_doc.auth_info_bytes.clone(),
            signatures: vec![signature],
        };

        proto_into_bytes(&tx_raw)
    }
}

#[derive(Clone, Debug)]
pub struct SequenceStrategy {
    pub kind: SequenceStrategyKind,
    pub value: Arc<AtomicU64>,
    pub has_queried: Arc<AtomicBool>,
}

impl SequenceStrategy {
    pub fn new(kind: SequenceStrategyKind) -> Self {
        Self {
            value: Arc::new(AtomicU64::new(match kind {
                SequenceStrategyKind::Query => 0,             // will be ignored
                SequenceStrategyKind::QueryAndIncrement => 0, // will be ignored
                SequenceStrategyKind::SetAndIncrement(n) => n,
                SequenceStrategyKind::Constant(n) => n,
            })),
            kind,
            has_queried: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum SequenceStrategyKind {
    /// Always query
    Query,
    /// Query the first time, and then increment each successful tx
    QueryAndIncrement,
    /// Set to this the first time, and then increment each successful tx
    SetAndIncrement(u64),
    /// Set to this each time
    Constant(u64),
}

pub enum FeeCalculation<'a> {
    Simulation {
        chain_config: &'a ChainConfig,
    },
    RealNetwork {
        chain_config: &'a ChainConfig,
        gas_units: u64,
    },
    RealCoin {
        gas_coin: layer_climb_proto::Coin,
        gas_units: u64,
    },
}

impl FeeCalculation<'_> {
    pub fn calculate(&self) -> Result<layer_climb_proto::tx::Fee> {
        let (gas_coin, gas_limit) = match self {
            Self::Simulation { chain_config } => (new_coin(0, &chain_config.gas_denom), 0),
            Self::RealNetwork {
                chain_config,
                gas_units,
            } => {
                let amount = (chain_config.gas_price * *gas_units as f32).ceil() as u128;
                (new_coin(amount, &chain_config.gas_denom), *gas_units)
            }
            Self::RealCoin {
                gas_coin,
                gas_units,
            } => (gas_coin.clone(), *gas_units),
        };

        Ok(layer_climb_proto::tx::Fee {
            amount: vec![gas_coin],
            gas_limit,
            payer: "".to_string(),
            granter: "".to_string(),
        })
    }
}

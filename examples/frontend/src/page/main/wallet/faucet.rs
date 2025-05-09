use crate::{
    client::{TargetEnvironment, ENVIRONMENT},
    prelude::*,
};
use dominator_helpers::futures::AsyncLoader;
use futures::StreamExt;
use gloo_timers::future::IntervalStream;
use wasm_bindgen_futures::spawn_local;

pub struct WalletFaucetUi {
    pub balance: Mutable<u128>,
    pub loader: AsyncLoader,
}

impl WalletFaucetUi {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            balance: Mutable::new(0),
            loader: AsyncLoader::new(),
        })
    }

    pub fn render(self: &Arc<Self>) -> Dom {
        let state = self;

        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("gap", "1rem")
            }
        });

        html!("div", {
            .class(&*CONTAINER)
            .future(clone!(state => async move {
                match client_event_receiver().recv().await {
                    Ok(ClientEvent::AddressChanged) => {
                        log::info!("address changed, updating balance immediately");
                        state.update_balance().await;
                    }
                    Err(err) => {
                        log::error!("Error receiving client event: {:?}", err);
                    }
                }
            }))
            .future(clone!(state => async move {
                state.update_balance().await;
                IntervalStream::new(3_000).for_each(clone!(state => move |_| clone!(state => async move {
                    state.update_balance().await;
                }))).await;
            }))
            .child(html!("div", {
                .class(&*TEXT_SIZE_XLG)
                .text_signal(state.balance.signal().map(clone!(state => move |balance| {
                    format!("Balance: {:.2}{}", balance, query_client().chain_config.gas_denom)
                })))
            }))
            .child(html!("div", {
                .child(Button::new()
                    .with_text("Tap it!")
                    .with_on_click(clone!(state => move || {
                        state.loader.load(clone!(state => {
                            async move {
                                if let Err(err) = state.get_tokens().await {
                                    log::error!("Error getting tokens: {:?}", err);
                                }
                            }
                        }));
                    }))
                    .render()
                )
            }))
            .child_signal(state.loader.is_loading().map(|is_loading| {
                match is_loading {
                    true => Some(html!("div", {
                        .class(&*TEXT_SIZE_MD)
                        .text("Getting tokens...")
                    })),
                    false => None
                }
            }))
        })
    }

    async fn update_balance(&self) {
        self.balance.set_neq(
            query_client()
                .balance(signing_client().addr.clone(), None)
                .await
                .unwrap_or_default()
                .unwrap_or_default(),
        );
    }

    async fn get_tokens(&self) -> Result<()> {
        tap_faucet(*ENVIRONMENT.get().unwrap(), &signing_client().addr).await?;

        self.update_balance().await;

        Ok(())
    }
}

pub async fn tap_faucet(environment: TargetEnvironment, addr: &Address) -> Result<()> {
    let chain_info = match environment {
        TargetEnvironment::Local => CONFIG
            .data
            .local
            .as_ref()
            .context("no local config found")?,

        TargetEnvironment::Testnet => CONFIG
            .data
            .testnet
            .as_ref()
            .context("no testnet config found")?,
    };

    let signer = KeySigner::new_mnemonic_str(&chain_info.faucet.mnemonic, None)?;
    let faucet = SigningClient::new(chain_info.chain.clone().into(), signer, None).await?;

    faucet.transfer(1_000_000, addr, None, None).await?;

    Ok(())
}

mod block;
mod contract;
mod sidebar;
mod topnav;
mod wallet;

use crate::prelude::*;
use block::events::BlockEventsUi;
use contract::{ContractExecuteUi, ContractInstantiateUi, ContractQueryUi, ContractUploadUi};
use wallet::faucet::WalletFaucetUi;

pub struct MainUi {}

impl MainUi {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub fn render(self: Arc<Self>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("height", "100vh")
                .style("background-color", Color::Background.hex_str())
            }
        });

        static SIDEBAR: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("flex-shrink", "0")
                .style("min-height", "100vh")
                .style("background-color", Color::Background.hex_str())
                .style("border-right", "1px solid")
                .style("border-color", Color::BorderPrimary.hex_str())
            }
        });

        static MAIN_CONTENT_WRAPPER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("flex-direction", "column")
                .style("flex-grow", "1")
                .style("overflow", "hidden")
            }
        });

        static TOPNAV: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("background-color", Color::Background.hex_str())
                .style("padding", "0 2rem")
                .style("flex-shrink", "0")
                .style("display", "flex")
                .style("align-items", "center")
            }
        });

        static CONTENT: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("flex-grow", "1")
                .style("overflow", "scroll")
                .style("padding", "2rem")
                .style("padding-top", "1rem")
            }
        });

        html!("div", {
            .class(&*CONTAINER)
            .children([
                html!("div", {
                    .class(&*SIDEBAR)
                    .child(sidebar::Sidebar::new().render())
                }),
                html!("div", {
                    .class(&*MAIN_CONTENT_WRAPPER)
                    .children([
                        html!("div", {
                            .class(&*TOPNAV)
                            .child(topnav::Topnav::new().render())
                        }),
                        html!("div", {
                            .class(&*CONTENT)
                            .child_signal(Route::signal().map(|route| {
                                match route {
                                    Route::WalletFaucet => Some(WalletFaucetUi::new().render()),
                                    Route::ContractUpload => Some(ContractUploadUi::new().render()),
                                    Route::ContractInstantiate => Some(ContractInstantiateUi::new().render()),
                                    Route::ContractExecute => Some(ContractExecuteUi::new().render()),
                                    Route::ContractQuery => Some(ContractQueryUi::new().render()),
                                    Route::BlockEvents => Some(BlockEventsUi::new().render()),
                                    _ => None,
                                }
                            }))
                        })
                    ])
                })
            ])
        })
    }
}

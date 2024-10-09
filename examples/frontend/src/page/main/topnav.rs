use std::sync::LazyLock;
use crate::{prelude::*};

pub struct Topnav {}

impl Topnav {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub fn render(self: Arc<Self>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("display", "flex")
                .style("align-items", "center")
                .style("padding", "20px 0")
                .style("width", "100%")
                .style("background-color", Color::Background.hex_str())
                .style("border-bottom", "1px solid")
                .style("border-color", Color::BorderPrimary.hex_str())
            }
        });

        static TITLE: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("font-weight", "600")
                .style("font-size", "20px")
                .style("color", Color::TextPrimary.hex_str())
                .style("flex-grow", "1")
                .style("text-align", "left")
            }
        });

        let title_signal = Route::signal().map(|selected_route| {
            match selected_route {
                Route::WalletFaucet => "Tap Faucet",
                Route::ContractUpload => "Contract Upload",
                Route::ContractInstantiate => "Contract Instantiate",
                Route::ContractExecute => "Contract Execute",
                Route::ContractQuery => "Contract Query",
                Route::BlockEvents => "Block Events",
                _ => "Climb",
            }
        });

        html!("div", {
            .class(&*CONTAINER)
            .child_signal(title_signal.map(|title| {
                Some(html!("div", {
                    .class(&*TITLE)
                    .text(title)
                }))
            }))
        })
    }
}

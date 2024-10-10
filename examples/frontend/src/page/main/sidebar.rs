use std::sync::LazyLock;

use wasm_bindgen_futures::spawn_local;

use crate::{prelude::*, util::mixins::handle_on_click};

pub struct Sidebar {}

impl Sidebar {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }

    pub fn render(self: Arc<Self>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
              .style("display", "flex")
              .style("flex-direction", "column")
              .style("gap", "1.3125rem")
              .style("align-items", "flex-start")
              .style("width", "302px")
              .style("padding", "20px 24px")
            }
        });

        static TITLE: LazyLock<String> = LazyLock::new(|| {
          class! {
            .style("font-weight", "600")
            .style("font-size", "20px")
            .style("width", "100%")
            .style("padding-bottom", "20px")
            .style("margin-bottom", "8px")
            .style("color", Color::TextPrimary.hex_str())
            .style("border-bottom", "1px solid")
            .style("border-color", Color::BorderPrimary.hex_str())
          }
        });

        html!("div", {
          .class(&*CONTAINER)
          .children([
            html!("div", {
              .class([&*TEXT_SIZE_XLG, &*TITLE])
              .text("Climb")
            }),
              self.render_section("Wallet", vec![
                  Route::WalletFaucet,
              ]),
              self.render_section("Contract", vec![
                  Route::ContractUpload,
                  Route::ContractInstantiate,
                  Route::ContractExecute,
                  Route::ContractQuery,
              ]),
              self.render_section("Block", vec![
                  Route::BlockEvents,
              ]),
          ])
        })
    }

    fn render_section(self: &Arc<Self>, title: &str, routes: Vec<Route>) -> Dom {
        static CONTAINER: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("width", "100%")
                .style("display", "flex")
                .style("flex-direction", "column")
            }
        });
        static TITLE: LazyLock<String> = LazyLock::new(|| {
            class! {
                .style("font-weight", "500")
                .style("font-size", "12px")
                .style("color", Color::TextSecondary.hex_str())
                .style("margin-bottom", "8px")
            }
        });

        let selected_sig = Route::signal().map(clone!(routes => move |selected_route| {
            routes.iter().any(|route| selected_route == *route)
        }));

        html!("div", {
            .class(&*CONTAINER)
            .children([
                html!("div", {
                    .class([&*TEXT_SIZE_XLG, &*TITLE, Color::Background.background_class()])
                    .text(title)
                }),
                html!("div", {
                    .style("width", "100%")
                    .children(routes.into_iter().map(|route| {
                        self.render_button(route)
                    }).collect::<Vec<Dom>>())
                })
            ])
        })
    }

    fn render_button(self: &Arc<Self>, route: Route) -> Dom {
        static BUTTON_BG_CLASS: LazyLock<String> = LazyLock::new(|| {
            class! {
              .style("cursor", "pointer")
              .style("display", "flex")
              .style("font-size", "15px")
              .style("font-weight", "600")
              .style("justify-content", "flex-start")
              .style("align-items", "center")
              .style("margin-bottom", "8px")
              .style("padding", "8px")
              .style("border-radius", "6px")
              .style("color", Color::TextPrimary.hex_str())
            }
        });

        static SELECTED: LazyLock<String> = LazyLock::new(|| {
            class! {
              .style("background-color", Color::BackgroundInteractiveSelected.hex_str())
            }
        });

        let selected_sig = Route::signal().map(clone!(route => move |selected_route| {
            selected_route == route
        }));

        html!("div", {
            .class([&*BUTTON_BG_CLASS, &*TEXT_SIZE_XLG, &*USER_SELECT_NONE])
            .class_signal([&*SELECTED, &*TEXT_WEIGHT_BOLD] , selected_sig)

            .text(match route {
                Route::WalletFaucet => "Tap Faucet",
                Route::ContractUpload => "Contract Upload",
                Route::ContractInstantiate => "Contract Instantiate",
                Route::ContractExecute => "Contract Execute",
                Route::ContractQuery => "Contract Query",
                Route::BlockEvents => "Block Events",
                _ => unreachable!(),
            })
            .apply(handle_on_click(move || {
                route.go_to_url();
            }))
        })
    }
}

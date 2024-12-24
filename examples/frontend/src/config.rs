use core::panic;

use awsm_web::env::{self, env_var};
use cosmwasm_std::Addr;

use crate::{
    client::{ClientKeyKind, TargetEnvironment},
    prelude::*,
    route::Route,
};

cfg_if::cfg_if! {
    if #[cfg(feature = "debug")] {

        pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
            Config {
                root_path: "",
                media_root: "http://localhost:9000",
                data: serde_json::from_str(include_str!("../../config.json")).unwrap_ext(),
                debug: ConfigDebug::dev_mode(),
            }
        });
    } else {
        pub static CONFIG: LazyLock<Config> = LazyLock::new(|| {
            Config {
                root_path: "climb",
                media_root: "https://lay3rlabs.github.io/climb/media",
                data: serde_json::from_str(include_str!("../../config.json")).unwrap_ext(),
                debug: ConfigDebug::default(),
            }
        });
    }
}

#[derive(Debug)]
pub struct Config {
    // the part of the url that is not the domain
    // e.g. in http://example.com/foo/bar, this would be "foo" if we want
    // all parsing to start from /bar
    // it's helpful in shared hosting environments where the app is not at the root
    pub root_path: &'static str,
    pub media_root: &'static str,
    pub debug: ConfigDebug,
    pub data: ConfigData,
}

impl Config {
    pub fn app_image_url(&self, path: &str) -> String {
        format!("{}/{}", self.media_root, path)
    }
}

#[derive(Debug)]
pub struct ConfigDebug {
    pub auto_connect: Option<ConfigDebugAutoConnect>,
    pub start_route: Mutex<Option<Route>>,
    pub contract_execute_address: Option<String>,
    pub contract_execute_message: Option<String>,
}

impl Default for ConfigDebug {
    fn default() -> Self {
        Self {
            auto_connect: None,
            start_route: Mutex::new(Some(Route::WalletFaucet)),
            contract_execute_address: None,
            contract_execute_message: None,
        }
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "autoconnect")] {
        impl ConfigDebug {
            pub fn dev_mode() -> Self {
                Self {
                    auto_connect: Some(ConfigDebugAutoConnect{
                        key_kind: ClientKeyKind::DirectEnv,
                        //key_kind: ClientKeyKind::Keplr,
                        target_env: TargetEnvironment::Local
                    }),
                    start_route: Mutex::new(Some(Route::BlockEvents)),
                    contract_execute_address: Some("layer1p8cyts27z9p022gu9z3qm7sxvdma0y09ur36u2nrhvjrdmmf38mqkq6kzz".to_string()),
                    contract_execute_message: Some(r#"{"stash_message": { "message": "hello world"}}"#.to_string()),
                    ..ConfigDebug::default()
                }
            }
        }
    } else {
        impl ConfigDebug {
            pub fn dev_mode() -> Self {
                Self {
                    auto_connect: None,
                    start_route: Mutex::new(Some(Route::ContractExecute)),
                    contract_execute_address: Some("layer1p8cyts27z9p022gu9z3qm7sxvdma0y09ur36u2nrhvjrdmmf38mqkq6kzz".to_string()),
                    contract_execute_message: Some(r#"{"stash_message": { "message": "hello world"}}"#.to_string()),
                    ..ConfigDebug::default()
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigDebugAutoConnect {
    pub key_kind: ClientKeyKind,
    pub target_env: TargetEnvironment,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigData {
    pub local: Option<ChainInfo>,
    pub testnet: Option<ChainInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChainInfo {
    pub chain: WebChainConfig,
    pub faucet: FaucetConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FaucetConfig {
    pub mnemonic: String,
}

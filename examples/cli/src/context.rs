use anyhow::{Context, Result};
use layer_climb::prelude::*;
use rand::rngs::ThreadRng;

use crate::{
    args::{CliArgs, TargetEnvironment},
    config::Config,
};

pub struct AppContext {
    pub args: CliArgs,
    pub config: Config,
    pub rng: ThreadRng,
}

impl AppContext {
    // Getting a context requires parsing the args first
    pub async fn new(args: CliArgs) -> Result<Self> {
        Ok(Self {
            args,
            config: Config::load()?,
            rng: ThreadRng::default(),
        })
    }

    pub fn chain_config(&self) -> Result<ChainConfig> {
        Ok(match self.args.target_env {
            TargetEnvironment::Local => self
                .config
                .local
                .as_ref()
                .context("no local config found")?
                .chain
                .clone(),
            TargetEnvironment::Testnet => self
                .config
                .local
                .as_ref()
                .context("no testnet config found")?
                .chain
                .clone(),
        })
    }

    pub async fn chain_querier(&self) -> Result<QueryClient> {
        QueryClient::new(self.chain_config()?, None).await
    }

    pub fn client_mnemonic(&self) -> Result<String> {
        let mnemonic_var = match self.args.target_env {
            TargetEnvironment::Local => "LOCAL_MNEMONIC",
            TargetEnvironment::Testnet => "TEST_MNEMONIC",
        };

        std::env::var(mnemonic_var)
            .and_then(|m| {
                if m.is_empty() {
                    Err(std::env::VarError::NotPresent)
                } else {
                    Ok(m)
                }
            })
            .context(format!("Mnemonic not found at {mnemonic_var}"))
    }

    // if we have a valid mnemonic, then get a signing client
    // otherwise, get a query client
    pub async fn any_client(&self) -> Result<AnyClient> {
        match self.client_mnemonic() {
            Ok(mnemonic) => {
                let signer = KeySigner::new_mnemonic_str(&mnemonic, None)?;
                Ok(AnyClient::Signing(
                    SigningClient::new(self.chain_config()?, signer, None).await?,
                ))
            }
            Err(_) => Ok(AnyClient::Query(self.chain_querier().await?)),
        }
    }

    pub async fn create_faucet(&self) -> Result<SigningClient> {
        let mnemonic = match self.args.target_env {
            TargetEnvironment::Local => {
                &self
                    .config
                    .local
                    .as_ref()
                    .context("no local config found")?
                    .faucet
                    .mnemonic
            }
            TargetEnvironment::Testnet => {
                &self
                    .config
                    .testnet
                    .as_ref()
                    .context("no testnet config found")?
                    .faucet
                    .mnemonic
            }
        };
        let signer = KeySigner::new_mnemonic_str(mnemonic, None)?;
        SigningClient::new(self.chain_config()?, signer, None).await
    }
}

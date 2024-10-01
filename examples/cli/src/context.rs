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
        match self.args.target_env {
            TargetEnvironment::Local => self.config.chains.local.clone(),
            TargetEnvironment::Testnet => self.config.chains.testnet.clone(),
        }
        .context(format!(
            "Chain config for environment {:?} not found",
            self.args.target_env
        ))
    }

    pub async fn chain_querier(&self) -> Result<QueryClient> {
        QueryClient::new(self.chain_config()?).await
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
                    SigningClient::new(self.chain_config()?, signer).await?,
                ))
            }
            Err(_) => Ok(AnyClient::Query(self.chain_querier().await?)),
        }
    }

    pub async fn create_faucet(&self) -> Result<SigningClient> {
        let signer = KeySigner::new_mnemonic_str(&self.config.faucet.mnemonic, None)?;
        SigningClient::new(self.chain_config()?, signer).await
    }
}

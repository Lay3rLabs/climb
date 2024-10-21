use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CliArgs {
    #[arg(long, default_value = "./config/faucet-layer-local.toml")]
    pub config: PathBuf,
}

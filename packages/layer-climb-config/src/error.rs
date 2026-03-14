use std::num::ParseFloatError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClimbConfigError {
    #[error("missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("invalid gas price: {0}")]
    InvalidGasPrice(#[from] ParseFloatError),

    #[error("invalid chain address kind: {0}")]
    InvalidAddressKind(String),

    #[error("missing bech32 prefix")]
    MissingBech32Prefix,

    #[error("failed to parse amount: {0}")]
    InvalidAmount(String),

    #[error("{0}")]
    Other(String),
}

impl ClimbConfigError {
    pub fn missing_env(var_name: impl Into<String>) -> Self {
        Self::MissingEnvVar(var_name.into())
    }

    pub fn invalid_address_kind(kind: impl Into<String>) -> Self {
        Self::InvalidAddressKind(kind.into())
    }
}

pub type Result<T> = std::result::Result<T, ClimbConfigError>;

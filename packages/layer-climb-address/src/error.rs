use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AddressError {
    #[error("invalid address format: {0}")]
    InvalidFormat(String),

    #[error("invalid address prefix: expected {expected}, got {actual}")]
    InvalidPrefix { expected: String, actual: String },

    #[error("address is not EVM")]
    NotEvm,

    #[error("address is not Cosmos")]
    NotCosmos,

    #[error("unsupported public key type")]
    UnsupportedPubKey,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AddressError>;

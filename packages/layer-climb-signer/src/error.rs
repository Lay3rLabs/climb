use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClimbSignerError {
    #[error("failed to sign: {0}")]
    SigningFailed(String),

    #[error("failed to get public key: {0}")]
    PublicKeyError(String),

    #[error("address error: {0}")]
    Address(#[from] layer_climb_address::ClimbAddressError),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ClimbSignerError>;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClimbSignerError {
    #[error("invalid public key type")]
    InvalidPublicKeyType,

    #[error("invalid derivation path: {0}")]
    InvalidDerivationPath(String),

    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(#[from] bip39::Error),

    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("signing failed: {0}")]
    SigningFailed(String),

    #[error("invalid secp256k1 public key: {0}")]
    InvalidSecp256k1PublicKey(String),

    #[error("keplr is only available in browsers")]
    KeplrNotAvailable,

    #[error("address error: {0}")]
    Address(#[from] layer_climb_address::ClimbAddressError),

    #[error("encoding error: {0}")]
    EncodeError(#[from] anyhow::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ClimbSignerError>;

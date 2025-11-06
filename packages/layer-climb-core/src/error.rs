use std::num::ParseFloatError;
use thiserror::Error;

/// The main error type for layer-climb operations.
///
/// This enum provides structured, recoverable error types that library consumers
/// can match against to handle specific error conditions.
#[derive(Error, Debug)]
pub enum ClimbError {
    /// Configuration-related errors
    #[error("configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Validation errors for user input
    #[error("validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Network and RPC-related errors
    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    /// Account and balance-related errors
    #[error("account error: {0}")]
    Account(#[from] AccountError),

    /// Transaction-related errors
    #[error("transaction error: {0}")]
    Transaction(#[from] TransactionError),

    /// Parsing errors
    #[error("parse error: {0}")]
    Parse(String),

    /// Resource not found
    #[error("not found: {0}")]
    NotFound(String),

    /// Generic error wrapper for compatibility with anyhow
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
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

/// Validation errors for user input
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("invalid denomination: expected {expected}, got {actual}")]
    InvalidDenom { expected: String, actual: String },

    #[error("invalid amount: {0}")]
    InvalidAmount(String),

    #[error("{0}")]
    Other(String),
}

/// Network and connectivity errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("failed to get client from pool: {0}")]
    ClientPool(String),

    #[error("RPC request failed: {0}")]
    Rpc(String),

    #[error("gRPC request failed: {0}")]
    Grpc(String),

    #[error("connection timeout: {0}")]
    Timeout(String),

    #[error("{0}")]
    Other(String),
}

/// Account-related errors
#[derive(Error, Debug)]
pub enum AccountError {
    #[error("account not found: {0}")]
    NotFound(String),

    #[error("insufficient balance: needed {needed}, available {available}")]
    InsufficientBalance { needed: String, available: String },

    #[error("{0}")]
    Other(String),
}

/// Transaction errors
#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("failed to build transaction: {0}")]
    BuildFailed(String),

    #[error("failed to sign transaction: {0}")]
    SigningFailed(String),

    #[error("transaction broadcast failed: {0}")]
    BroadcastFailed(String),

    #[error("transaction execution failed: {0}")]
    ExecutionFailed(String),

    #[error("{0}")]
    Other(String),
}

/// Type alias for Results using ClimbError
pub type Result<T> = std::result::Result<T, ClimbError>;

/// Helper functions for creating common errors
impl ClimbError {
    /// Create a parse error
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }
}

impl ConfigError {
    /// Create a missing environment variable error
    pub fn missing_env(var_name: impl Into<String>) -> Self {
        Self::MissingEnvVar(var_name.into())
    }

    /// Create an invalid address kind error
    pub fn invalid_address_kind(kind: impl Into<String>) -> Self {
        Self::InvalidAddressKind(kind.into())
    }
}

impl ValidationError {
    /// Create an invalid denom error
    pub fn invalid_denom(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::InvalidDenom {
            expected: expected.into(),
            actual: actual.into(),
        }
    }
}

impl NetworkError {
    /// Create a client pool error
    pub fn client_pool(msg: impl Into<String>) -> Self {
        Self::ClientPool(msg.into())
    }
}

impl AccountError {
    /// Create an account not found error
    pub fn not_found(address: impl Into<String>) -> Self {
        Self::NotFound(address.into())
    }
}

impl ClimbError {
    pub fn downcast_ref<T: std::error::Error + Send + Sync + 'static>(
        &self,
    ) -> std::result::Result<&T, ()> {
        match self {
            ClimbError::Other(e) => e.downcast_ref::<T>().ok_or(()),
            _ => panic!(
                "Cannot downcast structured ClimbError variants. Use pattern matching instead. \
                 Attempted to downcast {:?} to {}",
                self,
                std::any::type_name::<T>()
            ),
        }
    }
}

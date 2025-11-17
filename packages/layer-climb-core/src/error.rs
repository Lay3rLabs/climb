use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClimbError {
    #[error("address error: {0}")]
    Address(#[from] layer_climb_address::ClimbAddressError),

    #[error("config error: {0}")]
    Config(#[from] layer_climb_config::ClimbConfigError),

    #[error("signer error: {0}")]
    Signer(#[from] layer_climb_signer::ClimbSignerError),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, ClimbError>;

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

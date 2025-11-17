use axum::body::Body;
use axum::http::{Response, StatusCode};
use axum::response::IntoResponse;
use layer_climb::error::ClimbError;

pub type Result<T> = std::result::Result<T, AnyError>;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("not found")]
    NotFound,

    #[error("invalid denom: expected {expected}, got {got}")]
    InvalidDenom { expected: String, got: String },

    #[error("client pool error: {0}")]
    ClientPoolError(String),
}

pub struct AnyError(ClimbError);

impl IntoResponse for AnyError {
    fn into_response(self) -> Response<Body> {
        match self.0.downcast_ref::<AppError>() {
            Ok(app_error) => app_error.clone().into_response(),
            Err(_) => {
                let e = self.0.to_string();
                tracing::error!("{}", e);

                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(e.into())
                    .unwrap()
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let status = match &self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::InvalidDenom { .. } => StatusCode::BAD_REQUEST,
            AppError::ClientPoolError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = self.to_string().into();

        Response::builder()
            .status(status)
            .body(body)
            .expect("failed to render response")
    }
}

impl Clone for AppError {
    fn clone(&self) -> Self {
        match self {
            Self::NotFound => Self::NotFound,
            Self::InvalidDenom { expected, got } => Self::InvalidDenom {
                expected: expected.clone(),
                got: got.clone(),
            },
            Self::ClientPoolError(s) => Self::ClientPoolError(s.clone()),
        }
    }
}

impl From<ClimbError> for AnyError {
    fn from(err: ClimbError) -> Self {
        Self(err)
    }
}

impl From<anyhow::Error> for AnyError {
    fn from(err: anyhow::Error) -> Self {
        Self(ClimbError::Other(err))
    }
}

impl From<AppError> for AnyError {
    fn from(err: AppError) -> Self {
        Self(ClimbError::Other(err.into()))
    }
}

impl From<layer_climb::prelude::ClimbAddressError> for AnyError {
    fn from(err: layer_climb::prelude::ClimbAddressError) -> Self {
        Self(ClimbError::Address(err))
    }
}

impl From<layer_climb::prelude::ClimbSignerError> for AnyError {
    fn from(err: layer_climb::prelude::ClimbSignerError) -> Self {
        Self(ClimbError::Signer(err))
    }
}

impl From<layer_climb::prelude::ClimbConfigError> for AnyError {
    fn from(err: layer_climb::prelude::ClimbConfigError) -> Self {
        Self(ClimbError::Config(err))
    }
}

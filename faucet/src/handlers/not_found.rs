use crate::prelude::*;

pub async fn not_found() -> Result<()> {
    Err(AppError::NotFound.into())
}

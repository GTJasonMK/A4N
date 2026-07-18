use a4n_core::error::AppError;

mod indexes;
mod json;
mod text;

pub(super) use indexes::*;
pub(super) use json::*;
pub(super) use text::*;

pub(super) fn invalid(message: String) -> AppError {
    AppError::InvalidArgument(message)
}

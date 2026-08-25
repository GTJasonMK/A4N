pub(crate) mod object_profiles;

use a4n_core::error::AppError;
use a4n_db::asset::AssetDbError;

pub(crate) fn asset_error(error: AssetDbError) -> AppError {
    match error {
        AssetDbError::Database(error) => AppError::Database(error),
        AssetDbError::NotFound(message) => AppError::NotFound(message),
        AssetDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AssetDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
    }
}

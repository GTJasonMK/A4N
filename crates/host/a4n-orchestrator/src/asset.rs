pub(crate) mod object_profiles;

use a4n_db::asset::AssetDbError;
use a4n_error::AppError;

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

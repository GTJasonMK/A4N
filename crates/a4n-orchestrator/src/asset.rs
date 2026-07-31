mod object_profiles;

pub use a4n_db::asset::{AssetDbError, ObjectProfileUpsertRecordResult};
pub use object_profiles::{
    OBJECT_TYPES, ObjectProfileDraft, normalize_object_name, normalize_object_profile_record_input,
    normalize_object_profile_record_inputs, normalize_object_type_value,
};

use a4n_core::error::AppError;

pub(crate) fn asset_error(error: AssetDbError) -> AppError {
    match error {
        AssetDbError::Database(error) => AppError::Database(error),
        AssetDbError::NotFound(message) => AppError::NotFound(message),
        AssetDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
    }
}

use a4n_db::galgame::{
    GalgameAssetPlanSpecRecordError, GalgameContextRecordError, GalgameGenerationRecordError,
    GalgameLineRecordError, GalgameLineReorderRecordError, GalgameMediaRecordError,
    GalgameProjectRecordError, GalgameSceneRecordError, GalgameScriptRecordError,
};
use a4n_error::AppError;

pub(crate) fn galgame_project_record_error(err: GalgameProjectRecordError) -> AppError {
    match err {
        GalgameProjectRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameProjectRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameProjectRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_context_record_error(err: GalgameContextRecordError) -> AppError {
    match err {
        GalgameContextRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameContextRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameContextRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_script_record_error(err: GalgameScriptRecordError) -> AppError {
    match err {
        GalgameScriptRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameScriptRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_asset_plan_spec_record_error(
    err: GalgameAssetPlanSpecRecordError,
) -> AppError {
    match err {
        GalgameAssetPlanSpecRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        GalgameAssetPlanSpecRecordError::Serialization(err) => AppError::Serialization(err),
        GalgameAssetPlanSpecRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_generation_record_error(err: GalgameGenerationRecordError) -> AppError {
    match err {
        GalgameGenerationRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameGenerationRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        GalgameGenerationRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_media_record_error(err: GalgameMediaRecordError) -> AppError {
    match err {
        GalgameMediaRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameMediaRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameMediaRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_line_record_error(err: GalgameLineRecordError) -> AppError {
    match err {
        GalgameLineRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameLineRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameLineRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_line_reorder_record_error(err: GalgameLineReorderRecordError) -> AppError {
    match err {
        GalgameLineReorderRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        GalgameLineReorderRecordError::Database(err) => AppError::Database(err),
    }
}

pub(crate) fn galgame_scene_record_error(err: GalgameSceneRecordError) -> AppError {
    match err {
        GalgameSceneRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameSceneRecordError::Database(err) => AppError::Database(err),
    }
}

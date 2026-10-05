use a4n_db::galgame::{
    GalgameAssetPlanSpecRecordError, GalgameContextRecordError, GalgameGenerationRecordError,
    GalgameLineRecordError, GalgameLineReorderRecordError, GalgameMediaRecordError,
    GalgameProjectRecordError, GalgameSceneRecordError, GalgameScriptRecordError,
    GalgameStoryGraphRecordError,
};
use a4n_error::AppError;

pub(crate) fn galgame_project_record_error(err: GalgameProjectRecordError) -> AppError {
    match err {
        GalgameProjectRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameProjectRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameProjectRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_story_graph_record_error(err: GalgameStoryGraphRecordError) -> AppError {
    match err {
        GalgameStoryGraphRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        GalgameStoryGraphRecordError::RevisionConflict {
            expected_revision,
            current_revision,
        } => AppError::InvalidState {
            current: format!("story_graph_revision:{current_revision}"),
            operation: format!("使用 expected_revision={expected_revision} 保存节点图"),
        },
        GalgameStoryGraphRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_story_runtime_record_error(
    err: a4n_db::galgame::GalgameStoryRuntimeRecordError,
) -> AppError {
    match err {
        a4n_db::galgame::GalgameStoryRuntimeRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::galgame::GalgameStoryRuntimeRecordError::NotFound(message) => {
            AppError::NotFound(message)
        }
        a4n_db::galgame::GalgameStoryRuntimeRecordError::Conflict(message) => {
            AppError::InvalidState {
                current: message,
                operation: "更新 Galgame 运行存档".into(),
            }
        }
        a4n_db::galgame::GalgameStoryRuntimeRecordError::StoryGraph(error) => {
            galgame_story_graph_record_error(error)
        }
        a4n_db::galgame::GalgameStoryRuntimeRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_context_record_error(err: GalgameContextRecordError) -> AppError {
    match err {
        GalgameContextRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameContextRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameContextRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_script_record_error(err: GalgameScriptRecordError) -> AppError {
    match err {
        GalgameScriptRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameScriptRecordError::StoryGraph(error) => galgame_story_graph_record_error(error),
        GalgameScriptRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_asset_plan_spec_record_error(
    err: GalgameAssetPlanSpecRecordError,
) -> AppError {
    match err {
        GalgameAssetPlanSpecRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        GalgameAssetPlanSpecRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_generation_record_error(err: GalgameGenerationRecordError) -> AppError {
    match err {
        GalgameGenerationRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameGenerationRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        GalgameGenerationRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_media_record_error(err: GalgameMediaRecordError) -> AppError {
    match err {
        GalgameMediaRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameMediaRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameMediaRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_line_record_error(err: GalgameLineRecordError) -> AppError {
    match err {
        GalgameLineRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameLineRecordError::InvalidArgument(message) => AppError::InvalidArgument(message),
        GalgameLineRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_line_reorder_record_error(err: GalgameLineReorderRecordError) -> AppError {
    match err {
        GalgameLineReorderRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        GalgameLineReorderRecordError::Database(err) => AppError::database(err),
    }
}

pub(crate) fn galgame_scene_record_error(err: GalgameSceneRecordError) -> AppError {
    match err {
        GalgameSceneRecordError::NotFound(message) => AppError::NotFound(message),
        GalgameSceneRecordError::Database(err) => AppError::database(err),
    }
}

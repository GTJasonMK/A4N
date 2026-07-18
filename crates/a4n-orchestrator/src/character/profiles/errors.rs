use a4n_core::error::AppError;

use super::super::{
    CharacterPortraitRecordError, CharacterPortraitRollbackError, CharacterProfileCreateError,
};

pub(super) fn character_portrait_record_error(error: CharacterPortraitRecordError) -> AppError {
    match error {
        CharacterPortraitRecordError::NotFound(message) => AppError::NotFound(message),
        CharacterPortraitRecordError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        CharacterPortraitRecordError::Database(error) => AppError::Database(error),
    }
}

pub(super) fn character_profile_create_error(error: CharacterProfileCreateError) -> AppError {
    match error {
        CharacterProfileCreateError::AlreadyExists { name } => AppError::InvalidState {
            current: "character_profile_already_exists".to_string(),
            operation: format!("创建角色档案 {name}"),
        },
        CharacterProfileCreateError::InvalidArgument(message) => AppError::InvalidArgument(message),
        CharacterProfileCreateError::Database(error) => AppError::Database(error),
        CharacterProfileCreateError::Generation(error) => {
            crate::generation::generation_error(error)
        }
    }
}

pub(super) fn character_portrait_rollback_error(error: CharacterPortraitRollbackError) -> AppError {
    use CharacterPortraitRollbackError as Error;

    match error {
        Error::NotFound(message) => AppError::NotFound(message),
        Error::InvalidArgument(message) => AppError::InvalidArgument(message),
        Error::Begin(error)
        | Error::QueryPortrait(error)
        | Error::LockMirrorImages(error)
        | Error::DeleteMirrorImageBindings(error)
        | Error::DeleteMirrorImagePrimaryMaterials(error)
        | Error::DeleteMirrorImages(error)
        | Error::DeletePortrait(error)
        | Error::CheckFileReferences(error)
        | Error::Commit(error) => AppError::Database(error),
        Error::ClearMirrorImageReferences(error) => crate::video::video_db_error(error),
        Error::DeleteMirrorImageTargets(error) | Error::DeletePortraitTargets(error) => {
            crate::generation::generation_error(error)
        }
    }
}

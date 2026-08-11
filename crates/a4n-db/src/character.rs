mod attributes;
mod behavior;
mod blueprint_seed;
mod portrait;
mod profile;
mod snapshot;
pub mod snapshot_store;
mod sync_persistence;
mod timeline;

pub use attributes::*;
pub use behavior::*;
pub use blueprint_seed::*;
pub use portrait::*;
pub use profile::*;
pub use snapshot::*;
pub use sync_persistence::*;
pub use timeline::*;

use a4n_core::models::character::attribute_event::{
    SOURCE_KIND_AI_SYNC, SOURCE_KIND_LEGACY_UNKNOWN, SOURCE_KIND_USER,
};
use sea_orm::DbErr;

#[derive(Debug, thiserror::Error)]
pub enum CharacterDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
    #[error("{0}")]
    Internal(String),
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationDbError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

fn validate_category(category: &str) -> Result<(), CharacterDbError> {
    if matches!(category, "explicit" | "implicit" | "social") {
        Ok(())
    } else {
        Err(CharacterDbError::InvalidArgument(
            "属性分类必须是 explicit/implicit/social".to_string(),
        ))
    }
}

fn require_non_empty(value: &str, field_name: &str) -> Result<String, CharacterDbError> {
    let value = value.trim();
    if value.is_empty() {
        Err(CharacterDbError::InvalidArgument(format!(
            "{field_name}不能为空"
        )))
    } else {
        Ok(value.to_string())
    }
}

fn validate_chapter_number(chapter_number: i32) -> Result<(), CharacterDbError> {
    if chapter_number <= 0 {
        Err(CharacterDbError::InvalidArgument(
            "章节号必须大于 0".to_string(),
        ))
    } else {
        Ok(())
    }
}

pub(in crate::character) fn validate_timeline_source_kind(
    source_kind: &str,
) -> Result<String, CharacterDbError> {
    let source_kind = source_kind.trim();
    if matches!(
        source_kind,
        SOURCE_KIND_USER | SOURCE_KIND_AI_SYNC | SOURCE_KIND_LEGACY_UNKNOWN
    ) {
        Ok(source_kind.to_string())
    } else {
        Err(CharacterDbError::InvalidArgument(format!(
            "角色时间线 source_kind 无效: {source_kind}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn character_db_error_preserves_structured_classification() {
        let invalid_argument = CharacterDbError::InvalidArgument("章节范围无效".to_string());
        assert!(matches!(
            invalid_argument,
            CharacterDbError::InvalidArgument(message) if message == "章节范围无效"
        ));

        let database =
            CharacterDbError::Database(DbErr::Custom("timeline query failed".to_string()));
        assert!(matches!(
            database,
            CharacterDbError::Database(DbErr::Custom(message))
                if message == "timeline query failed"
        ));

        let generation =
            CharacterDbError::from(crate::generation::GenerationDbError::InvalidArgument(
                "generation contract failed".to_string(),
            ));
        assert!(matches!(
            generation,
            CharacterDbError::Generation(
                crate::generation::GenerationDbError::InvalidArgument(message)
            ) if message == "generation contract failed"
        ));

        let serialization = CharacterDbError::from(
            serde_json::from_str::<Value>("{").expect_err("invalid JSON must fail"),
        );
        assert!(matches!(serialization, CharacterDbError::Serialization(_)));
    }
}

use super::super::{SourceSyncDbError, validate_positive_id};
use crate::project::DerivedChapterScope;
use a4n_core::models::project::project_derivation;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use serde_json::Value;

pub async fn source_sync_chapter_scope_for_derived_project<C>(
    db: &C,
    derived_project_id: i64,
) -> Result<DerivedChapterScope, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(derived_project_id, "derived_project_id")?;
    let derivation = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.eq(derived_project_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::InvalidArgument(format!(
                "派生项目缺少派生关系: {derived_project_id}"
            ))
        })?;
    parse_source_sync_chapter_scope(&derivation.chapter_scope_json)
}

pub(in crate::source_sync) fn parse_source_sync_chapter_scope(
    value: &Value,
) -> Result<DerivedChapterScope, SourceSyncDbError> {
    let start = value.get("start").and_then(Value::as_i64).ok_or_else(|| {
        SourceSyncDbError::InvalidArgument("chapter_scope_json.start 缺失".to_string())
    })?;
    let end = value.get("end").and_then(Value::as_i64).ok_or_else(|| {
        SourceSyncDbError::InvalidArgument("chapter_scope_json.end 缺失".to_string())
    })?;
    if start <= 0 || end < start || start > i32::MAX as i64 || end > i32::MAX as i64 {
        return Err(SourceSyncDbError::InvalidArgument(
            "chapter_scope_json 范围无效".to_string(),
        ));
    }
    Ok(DerivedChapterScope {
        start_chapter: i32::try_from(start).map_err(|_| {
            SourceSyncDbError::InvalidArgument("chapter_scope_json.start 超出 i32 范围".to_string())
        })?,
        end_chapter: i32::try_from(end).map_err(|_| {
            SourceSyncDbError::InvalidArgument("chapter_scope_json.end 超出 i32 范围".to_string())
        })?,
    })
}

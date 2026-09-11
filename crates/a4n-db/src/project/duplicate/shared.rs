use a4n_db_models::models::project::novel_project;
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use std::collections::HashMap;

use super::ProjectDuplicateError;

#[derive(Debug, Clone)]
pub(super) struct ProjectCopyInsert {
    pub(super) user_id: i64,
    pub(super) title: String,
    pub(super) root_project_id: Option<i64>,
    pub(super) source_snapshot_id: Option<i64>,
    pub(super) status: String,
    pub(super) current_chapter: i32,
    pub(super) total_chapters: Option<i32>,
    pub(super) raw_blueprint: Option<String>,
}

pub(super) async fn insert_project_copy_record<C>(
    db: &C,
    original: &novel_project::Model,
    values: ProjectCopyInsert,
    now: NaiveDateTime,
) -> Result<novel_project::Model, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    novel_project::ActiveModel {
        user_id: Set(values.user_id),
        title: Set(values.title),
        description: Set(original.description.clone()),
        project_type: Set(original.project_type.clone()),
        project_kind: Set(original.project_kind.clone()),
        root_project_id: Set(values.root_project_id),
        source_snapshot_id: Set(values.source_snapshot_id),
        source_sync_status: Set(original.source_sync_status.clone()),
        status: Set(values.status),
        target_chapters: Set(original.target_chapters),
        current_chapter: Set(values.current_chapter),
        total_chapters: Set(values.total_chapters),
        chapters_per_part: Set(original.chapters_per_part),
        avatar_svg: Set(original.avatar_svg.clone()),
        avatar_animal: Set(original.avatar_animal.clone()),
        raw_blueprint: Set(values.raw_blueprint),
        import_analysis_progress: Set(None),
        export_config: Set(original.export_config.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectDuplicateError::Database)
}

pub(super) fn remap_derived_required_id(
    map: &HashMap<i64, i64>,
    old_id: i64,
    field: &str,
) -> Result<i64, ProjectDuplicateError> {
    map.get(&old_id)
        .copied()
        .ok_or_else(|| ProjectDuplicateError::InvalidState {
            current: format!("{field}={old_id} 缺少目标映射"),
            operation: "复制派生项目引用图".to_string(),
        })
}

pub(super) fn remap_derived_optional_id(
    map: &HashMap<i64, i64>,
    old_id: Option<i64>,
    field: &str,
) -> Result<Option<i64>, ProjectDuplicateError> {
    old_id
        .map(|id| remap_derived_required_id(map, id, field))
        .transpose()
}

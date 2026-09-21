use a4n_db_models::models::{
    generation::generation_spec, project::project_source_snapshot, source_sync::source_sync_run,
    video_project::video_segment,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
};
use serde_json::Value;

use super::contracts::SourceSyncReplacementSegmentRecordInput;
use crate::source_sync::{
    SourceSyncDbError, required_owned_text, source_snapshot_chapter_id_for_source,
    validate_positive_id,
};

pub(super) struct PreparedReplacementGenerationSpec {
    pub(super) source_chapter_id: i64,
    pub(super) snapshot_chapter_id: i64,
    pub(super) snapshot_id: i64,
    pub(super) source_version_hash: String,
    pub(super) title: String,
}

pub(super) async fn prepare_replacement_generation_spec<C>(
    db: &C,
    run: &source_sync_run::Model,
    source_spec: &generation_spec::Model,
    params_json: &Value,
    replacement_segment: &SourceSyncReplacementSegmentRecordInput,
) -> Result<PreparedReplacementGenerationSpec, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;
    validate_positive_id(source_spec.id, "source_generation_spec_id")?;
    if source_spec.project_id != run.derived_project_id
        || source_spec.root_project_id != run.root_project_id
    {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "生成意图 {} 不属于本次源同步运行",
            source_spec.id
        )));
    }
    if source_spec.target_entity_kind != "segment" {
        return Err(SourceSyncDbError::InvalidArgument(
            "create_replacement 只支持视频片段生成意图".to_string(),
        ));
    }
    let source_chapter_id = source_spec.source_chapter_id.ok_or_else(|| {
        SourceSyncDbError::InvalidArgument(format!(
            "生成意图 {} 缺少 source_chapter_id",
            source_spec.id
        ))
    })?;
    let snapshot_chapter_id =
        source_snapshot_chapter_id_for_source(db, run.to_snapshot_id, source_chapter_id).await?;
    let source_version_hash = project_source_snapshot::Entity::find_by_id(run.to_snapshot_id)
        .select_only()
        .column(project_source_snapshot::Column::SourceVersionHash)
        .into_tuple::<String>()
        .one(db)
        .await?
        .ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("目标源快照不存在: {}", run.to_snapshot_id))
        })?;
    if !params_json.is_object() {
        return Err(SourceSyncDbError::InvalidArgument(
            "params_json 必须是 JSON 对象".to_string(),
        ));
    }
    let title = required_owned_text(
        replacement_segment.title.clone(),
        "replacement_segment.title",
    )?;
    if replacement_segment.duration_secs <= 0.0 {
        return Err(SourceSyncDbError::InvalidArgument(
            "replacement_segment.duration_secs 必须大于 0".to_string(),
        ));
    }
    if let Some(previous_segment_id) = replacement_segment.previous_segment_id {
        ensure_video_segment_in_project(db, previous_segment_id, source_spec.project_id).await?;
    }
    if let Some(next_segment_id) = replacement_segment.next_segment_id {
        ensure_video_segment_in_project(db, next_segment_id, source_spec.project_id).await?;
    }

    Ok(PreparedReplacementGenerationSpec {
        source_chapter_id,
        snapshot_chapter_id,
        snapshot_id: run.to_snapshot_id,
        source_version_hash,
        title,
    })
}

async fn ensure_video_segment_in_project<C>(
    db: &C,
    segment_id: i64,
    project_id: i64,
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(segment_id, "segment_id")?;
    validate_positive_id(project_id, "project_id")?;
    let count = video_segment::Entity::find_by_id(segment_id)
        .filter(video_segment::Column::ProjectId.eq(project_id))
        .count(db)
        .await?;
    if count == 0 {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "视频片段 {} 不属于视频项目 {}",
            segment_id, project_id
        )));
    }
    Ok(())
}

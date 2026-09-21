use a4n_db_models::models::{
    generation::generation_spec, source_sync::source_sync_run, video_project::video_segment,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::Value;

use super::contracts::SourceSyncReplacementSegmentRecordInput;
use super::preparation::PreparedReplacementGenerationSpec;
use crate::source_sync::{
    SourceSyncDbError, SourceSyncEntityRefRecordInput, SourceSyncMediaRefRecordInput,
    SourceSyncTextAnchorRefRecordInput, insert_generation_spec_ref_records,
    source_sync_generation_db_error,
};

pub(super) async fn persist_replacement_generation_spec<C>(
    db: &C,
    run: &source_sync_run::Model,
    source_spec: &generation_spec::Model,
    params_json: Value,
    replacement_segment: SourceSyncReplacementSegmentRecordInput,
    text_anchor_refs: Vec<SourceSyncTextAnchorRefRecordInput>,
    entity_refs: Vec<SourceSyncEntityRefRecordInput>,
    media_refs: Vec<SourceSyncMediaRefRecordInput>,
    prepared: PreparedReplacementGenerationSpec,
) -> Result<generation_spec::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let segment = video_segment::ActiveModel {
        project_id: Set(source_spec.project_id),
        root_project_id: Set(source_spec.root_project_id),
        source_chapter_id: Set(Some(prepared.source_chapter_id)),
        source_snapshot_chapter_id: Set(Some(prepared.snapshot_chapter_id)),
        segment_type: Set("replacement".to_string()),
        order_index: Set(replacement_segment.order_index),
        title: Set(prepared.title),
        duration_secs: Set(replacement_segment.duration_secs),
        source_generation_spec_id: Set(None),
        result_media_asset_id: Set(None),
        previous_segment_id: Set(replacement_segment.previous_segment_id),
        next_segment_id: Set(replacement_segment.next_segment_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let spec = generation_spec::ActiveModel {
        project_id: Set(source_spec.project_id),
        root_project_id: Set(source_spec.root_project_id),
        source_context_kind: Set(Some("snapshot_backed".to_string())),
        source_snapshot_id: Set(Some(prepared.snapshot_id)),
        source_chapter_id: Set(Some(prepared.source_chapter_id)),
        source_snapshot_chapter_id: Set(Some(prepared.snapshot_chapter_id)),
        target_kind: Set(source_spec.target_kind.clone()),
        target_entity_kind: Set("segment".to_string()),
        target_entity_id: Set(Some(segment.id)),
        status: Set("draft".to_string()),
        source_version_hash: Set(prepared.source_version_hash),
        params_json: Set(params_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    insert_generation_spec_ref_records(
        db,
        &spec,
        run.to_snapshot_id,
        text_anchor_refs,
        entity_refs,
        media_refs,
    )
    .await?;

    let mut active_segment: video_segment::ActiveModel = segment.into();
    active_segment.source_generation_spec_id = Set(Some(spec.id));
    active_segment.updated_at = Set(now);
    active_segment.update(db).await?;

    crate::generation::validate_required_prompt_refs(db, &spec)
        .await
        .map_err(source_sync_generation_db_error)?;
    Ok(spec)
}

use super::references::generation_spec_id_for_source_sync_run;
use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_db_models::models::{
    generation::prompt_artifact,
    source_sync::source_sync_run,
    video_project::{assembly_clip, video_segment},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashSet;

pub async fn delete_generation_spec_records<C>(
    db: &C,
    generation_spec_ids: &[i64],
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(());
    }
    for generation_spec_id in generation_spec_ids {
        validate_positive_id(*generation_spec_id, "generation_spec_id")?;
    }
    let generation_spec_ids = generation_spec_ids.to_vec();
    let prompt_artifact_ids = prompt_artifact::Entity::find()
        .select_only()
        .column(prompt_artifact::Column::Id)
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
        .into_tuple::<i64>()
        .all(db)
        .await?;

    crate::generation::delete_generation_spec_records_by_ids(
        db,
        generation_spec_ids,
        prompt_artifact_ids,
    )
    .await?;
    Ok(())
}

pub async fn delete_local_generation_spec_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    generation_spec_id: i64,
) -> Result<(), SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let spec_id = generation_spec_id_for_source_sync_run(db, run, generation_spec_id).await?;
    let segments = video_segment::Entity::find()
        .select_only()
        .column(video_segment::Column::Id)
        .column(video_segment::Column::ResultMediaAssetId)
        .filter(video_segment::Column::SourceGenerationSpecId.eq(spec_id))
        .into_tuple::<(i64, Option<i64>)>()
        .all(db)
        .await?;
    let segment_ids = segments.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let timeline_segment_ids = if segment_ids.is_empty() {
        HashSet::new()
    } else {
        assembly_clip::Entity::find()
            .select_only()
            .column(assembly_clip::Column::SourceSegmentId)
            .filter(assembly_clip::Column::SourceSegmentId.is_in(segment_ids.clone()))
            .into_tuple::<Option<i64>>()
            .all(db)
            .await?
            .into_iter()
            .flatten()
            .collect()
    };
    for (segment_id, result_media_asset_id) in &segments {
        if result_media_asset_id.is_some() {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "视频片段 {} 已有结果媒体，不能 delete_local",
                segment_id
            )));
        }
        if timeline_segment_ids.contains(segment_id) {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "视频片段 {} 已进入时间线，不能 delete_local",
                segment_id
            )));
        }
    }
    if !segment_ids.is_empty() {
        video_segment::Entity::delete_many()
            .filter(video_segment::Column::Id.is_in(segment_ids))
            .exec(db)
            .await?;
    }
    delete_generation_spec_records(db, &[spec_id]).await
}

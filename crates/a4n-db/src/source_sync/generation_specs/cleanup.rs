use super::references::generation_spec_for_source_sync_run;
use crate::source_sync::{SourceSyncDbError, validate_positive_id};
use a4n_core::models::{
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::media_provenance,
    source_sync::source_sync_run,
    video_project::{assembly_clip, video_segment},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
    sea_query::Expr,
};

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

    let mut provenance_update = media_provenance::Entity::update_many();
    provenance_update = provenance_update
        .col_expr(
            media_provenance::Column::GenerationSpecId,
            Expr::value(Option::<i64>::None),
        )
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.clone()));
    if !prompt_artifact_ids.is_empty() {
        provenance_update = provenance_update.col_expr(
            media_provenance::Column::PromptArtifactId,
            Expr::value(Option::<i64>::None),
        );
    }
    provenance_update.exec(db).await?;

    if !prompt_artifact_ids.is_empty() {
        media_provenance::Entity::update_many()
            .col_expr(
                media_provenance::Column::PromptArtifactId,
                Expr::value(Option::<i64>::None),
            )
            .filter(media_provenance::Column::PromptArtifactId.is_in(prompt_artifact_ids))
            .exec(db)
            .await?;
    }
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    generation_spec_text_anchor::Entity::delete_many()
        .filter(
            generation_spec_text_anchor::Column::GenerationSpecId
                .is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(
            generation_spec_entity_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    prompt_artifact::Entity::delete_many()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
        .exec(db)
        .await?;
    generation_spec::Entity::delete_many()
        .filter(generation_spec::Column::Id.is_in(generation_spec_ids))
        .exec(db)
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
    let spec = generation_spec_for_source_sync_run(db, run, generation_spec_id).await?;
    let segments = video_segment::Entity::find()
        .filter(video_segment::Column::SourceGenerationSpecId.eq(spec.id))
        .all(db)
        .await?;
    for segment in &segments {
        if segment.result_media_asset_id.is_some() {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "视频片段 {} 已有结果媒体，不能 delete_local",
                segment.id
            )));
        }
        let clip_count = assembly_clip::Entity::find()
            .filter(assembly_clip::Column::SourceSegmentId.eq(segment.id))
            .count(db)
            .await?;
        if clip_count > 0 {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "视频片段 {} 已进入时间线，不能 delete_local",
                segment.id
            )));
        }
    }
    for segment in segments {
        video_segment::Entity::delete_by_id(segment.id)
            .exec(db)
            .await?;
    }
    delete_generation_spec_records(db, &[spec.id]).await
}

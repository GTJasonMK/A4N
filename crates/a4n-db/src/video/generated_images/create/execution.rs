use super::super::{
    CreateGeneratedImageRecordDraftInput, VideoDbError, lock_generated_media_project,
};
use super::record::insert_generated_image_record;
use super::trace::{image_generation_parsed_output, image_generation_response_summary};
use super::validation::validate_create_generated_image_record_input;
use a4n_core::models::ai::generated_image;
use sea_orm::{ConnectionTrait, TransactionTrait};

fn generated_image_target(
    image: &generated_image::Model,
) -> crate::generation::GenerationRunTraceTarget {
    crate::generation::GenerationRunTraceTarget {
        target_kind: image.image_type.clone(),
        target_table: "generated_image".to_string(),
        target_id: image.id,
    }
}

pub async fn create_generated_image_record<C>(
    db: &C,
    input: CreateGeneratedImageRecordDraftInput,
) -> Result<generated_image::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, input.project_id).await?;
    let input = validate_create_generated_image_record_input(&txn, input).await?;
    let image = insert_generated_image_record(&txn, input)
        .await
        .map_err(VideoDbError::Database)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(image)
}

/// 创建图片并写入 completed generation run checkpoint。
pub async fn create_generated_image_record_with_generation_run_checkpoint<C>(
    db: &C,
    input: CreateGeneratedImageRecordDraftInput,
    mut generation_run: crate::generation::RecordCompletedGenerationRunWithCheckpointInput,
) -> Result<generated_image::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, input.project_id).await?;
    let input = validate_create_generated_image_record_input(&txn, input).await?;
    if generation_run.run.project_id != input.project_id {
        return Err(VideoDbError::InvalidArgument(
            "图片生成运行记录 project_id 与图片资产不一致".to_string(),
        ));
    }
    let image = insert_generated_image_record(&txn, input)
        .await
        .map_err(VideoDbError::Database)?;
    let target = generated_image_target(&image);
    generation_run.run.targets.push(target.clone());
    generation_run.run.response_summary_json =
        image_generation_response_summary(generation_run.run.response_summary_json, &image)?;
    generation_run.checkpoint.parsed_output_json =
        image_generation_parsed_output(generation_run.checkpoint.parsed_output_json, &image)?;
    generation_run.checkpoint.artifact_refs_json = Some(
        crate::generation::append_generation_artifact_target(
            generation_run.checkpoint.artifact_refs_json,
            target,
        )
        .map_err(VideoDbError::from)?,
    );
    crate::generation::record_completed_generation_run_with_checkpoint_in(
        &txn,
        generation_run.run,
        generation_run.checkpoint,
    )
    .await
    .map_err(VideoDbError::from)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(image)
}

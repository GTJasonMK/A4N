use super::super::records::mark_prompt_video_generation_running_for_video_record;
use super::generation::{record_video_generation_run, validate_generation_run_for_video};
use super::record::insert_generated_video_record;
use super::validation::validate_create_generated_video_record_input;
use crate::generation::RecordStartedGenerationRunInput;
use crate::video::{VideoDbError, lock_generated_media_project};
use a4n_domain::media::{CreateGeneratedVideoRecordDraftInput, GeneratedVideoRecord};
use sea_orm::{ConnectionTrait, TransactionTrait};

use crate::video::media_mapping::generated_video_record_from_model;

/// 校验项目、章节和源媒体归属后写入 generated_video。
pub async fn create_generated_video_record<C>(
    db: &C,
    input: CreateGeneratedVideoRecordDraftInput,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, input.project_id).await?;
    let input = validate_create_generated_video_record_input(&txn, input).await?;
    let video = insert_generated_video_record(&txn, input)
        .await
        .map_err(VideoDbError::Database)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(generated_video_record_from_model(video))
}

/// 创建视频并绑定 generation run（同一事务）。
pub async fn create_generated_video_record_with_generation_run<C>(
    db: &C,
    input: CreateGeneratedVideoRecordDraftInput,
    generation_run: RecordStartedGenerationRunInput,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, input.project_id).await?;
    let input = validate_create_generated_video_record_input(&txn, input).await?;
    validate_generation_run_for_video(&input, &generation_run)?;
    let video = insert_generated_video_record(&txn, input)
        .await
        .map_err(VideoDbError::Database)?;
    record_video_generation_run(&txn, &video, generation_run).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(generated_video_record_from_model(video))
}

/// 创建提示词联动视频并绑定 generation run（同一事务）。
pub async fn create_prompt_generated_video_record_with_generation_run<C>(
    db: &C,
    input: CreateGeneratedVideoRecordDraftInput,
    generation_run: RecordStartedGenerationRunInput,
    prompt_id: i64,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, input.project_id).await?;
    let input = validate_create_generated_video_record_input(&txn, input).await?;
    validate_generation_run_for_video(&input, &generation_run)?;
    let video = insert_generated_video_record(&txn, input)
        .await
        .map_err(VideoDbError::Database)?;
    mark_prompt_video_generation_running_for_video_record(&txn, prompt_id, &video).await?;
    record_video_generation_run(&txn, &video, generation_run).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(generated_video_record_from_model(video))
}

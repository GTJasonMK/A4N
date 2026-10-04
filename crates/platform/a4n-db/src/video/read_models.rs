use a4n_db_models::models::ai::generated_image;
use a4n_domain::media::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedVideoRecordDraftInput,
    GeneratedImageRecord, GeneratedImageReferenceRecord, GeneratedVideoErrorStatusBatchInput,
    GeneratedVideoRecord, ImagePublicationFileFacts, UpdateGeneratedImageRecordInput,
    UpdateGeneratedVideoRecordInput, VideoFileProperties,
};
use a4n_domain::video_prompt::{
    ChapterVideoPromptPatch, ChapterVideoPromptRecord, ChapterVideoPromptSyncDraft,
    ChapterVideoPromptSyncPolicy, UpdateChapterVideoPromptRecordInput,
};
use sea_orm::{ConnectionTrait, DbErr, TransactionTrait};
use serde_json::Value;

use super::VideoDbError;
use super::media_mapping::{
    generated_image_record_from_model, generated_image_reference_record_from_model,
    generated_video_record_from_model,
};
use super::prompts::record_mapping::{
    chapter_video_prompt_model_from_record, chapter_video_prompt_record_from_model,
};
use crate::generation::{
    RecordCompletedGenerationRunWithCheckpointInput, RecordStartedGenerationRunInput,
};

fn image_records(images: Vec<generated_image::Model>) -> Vec<GeneratedImageRecord> {
    images
        .into_iter()
        .map(generated_image_record_from_model)
        .collect()
}

fn image_reference_records(
    images: Vec<generated_image::Reference>,
) -> Vec<GeneratedImageReferenceRecord> {
    images
        .into_iter()
        .map(generated_image_reference_record_from_model)
        .collect()
}

fn chapter_video_prompt_records(
    prompts: Vec<a4n_db_models::models::chapter::chapter_video_prompt::Model>,
) -> Vec<ChapterVideoPromptRecord> {
    prompts
        .into_iter()
        .map(chapter_video_prompt_record_from_model)
        .collect()
}

pub(crate) async fn generated_images_for_chapter_by_id_desc<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedImageReferenceRecord>, DbErr> {
    super::generated_images_for_chapter_by_id_desc(db, project_id, chapter_id)
        .await
        .map(image_reference_records)
}

pub async fn generated_images_for_projects_chapter_by_id_desc<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
    chapter_id: i64,
) -> Result<Vec<GeneratedImageReferenceRecord>, DbErr> {
    super::generated_images_for_projects_chapter_by_id_desc(db, project_ids, chapter_id)
        .await
        .map(image_reference_records)
}

pub async fn generated_image_by_id<C: ConnectionTrait>(
    db: &C,
    image_id: i64,
) -> Result<Option<GeneratedImageRecord>, DbErr> {
    super::generated_image_by_id(db, image_id)
        .await
        .map(|image| image.map(generated_image_record_from_model))
}

pub async fn generated_images_by_ids(
    db: &impl crate::DbExecutor,
    image_ids: &[i64],
) -> Result<Vec<GeneratedImageRecord>, VideoDbError> {
    super::generated_images_by_ids(db, image_ids)
        .await
        .map(image_records)
}

pub async fn generated_images_for_project<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
) -> Result<Vec<GeneratedImageRecord>, DbErr> {
    super::generated_images_for_project(db, project_id)
        .await
        .map(image_records)
}

pub async fn generated_images_for_projects<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<GeneratedImageRecord>, VideoDbError> {
    super::generated_images_for_projects(db, project_ids)
        .await
        .map(image_records)
}

pub async fn generated_images_for_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedImageRecord>, DbErr> {
    super::generated_images_for_chapter(db, project_id, chapter_id)
        .await
        .map(image_records)
}

pub async fn video_library_explicit_image_records<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
    image_ids: &[i64],
) -> Result<Vec<GeneratedImageRecord>, DbErr> {
    super::video_library_explicit_image_records(db, project_ids, image_ids)
        .await
        .map(image_records)
}

pub async fn video_library_chapter_image_records<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
    chapter_id: i64,
) -> Result<Vec<GeneratedImageRecord>, DbErr> {
    super::video_library_chapter_image_records(db, project_ids, chapter_id)
        .await
        .map(image_records)
}

pub async fn portrait_reference_images_by_ids<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    image_ids: &[i64],
) -> Result<Vec<GeneratedImageRecord>, crate::character::CharacterPortraitRecordError> {
    crate::character::portrait_reference_images_by_ids(db, project_id, image_ids)
        .await
        .map(image_records)
}

pub async fn validated_video_reference_image_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: Option<i64>,
    reference_image_ids_json: Option<&Value>,
) -> Result<Vec<GeneratedImageRecord>, VideoDbError> {
    super::validated_video_reference_image_records(
        db,
        project_id,
        chapter_id,
        reference_image_ids_json,
    )
    .await
    .map(image_records)
}

pub async fn video_reference_candidate_image_records<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
    chapter_id: i64,
) -> Result<Vec<GeneratedImageRecord>, VideoDbError> {
    super::video_reference_candidate_image_records(db, project_ids, chapter_id)
        .await
        .map(image_records)
}

pub async fn update_generated_image_record<C: ConnectionTrait>(
    db: &C,
    image_id: i64,
    input: UpdateGeneratedImageRecordInput,
) -> Result<GeneratedImageRecord, VideoDbError> {
    super::update_generated_image_record(db, image_id, input)
        .await
        .map(generated_image_record_from_model)
}

pub async fn create_generated_image_record<C>(
    db: &C,
    input: CreateGeneratedImageRecordDraftInput,
) -> Result<GeneratedImageRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::create_generated_image_record(db, input)
        .await
        .map(generated_image_record_from_model)
}

pub async fn publish_image_publication<C: ConnectionTrait + TransactionTrait>(
    db: &C,
    publication_id: &str,
    facts: ImagePublicationFileFacts,
) -> Result<GeneratedImageRecord, VideoDbError> {
    super::publish_image_publication(db, publication_id, facts)
        .await
        .map(generated_image_record_from_model)
}

pub async fn publish_image_publication_with_generation_run_checkpoint<C>(
    db: &C,
    publication_id: &str,
    facts: ImagePublicationFileFacts,
    generation_run: RecordCompletedGenerationRunWithCheckpointInput,
) -> Result<GeneratedImageRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::publish_image_publication_with_generation_run_checkpoint(
        db,
        publication_id,
        facts,
        generation_run,
    )
    .await
    .map(generated_image_record_from_model)
}

pub async fn generated_video_by_id<C: ConnectionTrait>(
    db: &C,
    video_id: i64,
) -> Result<Option<GeneratedVideoRecord>, DbErr> {
    super::generated_video_by_id(db, video_id)
        .await
        .map(|video| video.map(generated_video_record_from_model))
}

pub async fn generated_videos_for_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedVideoRecord>, DbErr> {
    super::generated_videos_for_chapter(db, project_id, chapter_id)
        .await
        .map(|videos| {
            videos
                .into_iter()
                .map(generated_video_record_from_model)
                .collect()
        })
}

pub async fn generated_videos_by_ids<C: ConnectionTrait>(
    db: &C,
    video_ids: &[i64],
) -> Result<Vec<GeneratedVideoRecord>, DbErr> {
    super::generated_videos_by_ids(db, video_ids)
        .await
        .map(|videos| {
            videos
                .into_iter()
                .map(generated_video_record_from_model)
                .collect()
        })
}

pub async fn generated_videos_by_provider_job_id<C: ConnectionTrait>(
    db: &C,
    project_id: Option<i64>,
    provider_job_id: &str,
) -> Result<Vec<GeneratedVideoRecord>, DbErr> {
    super::generated_videos_by_provider_job_id(db, project_id, provider_job_id)
        .await
        .map(|videos| {
            videos
                .into_iter()
                .map(generated_video_record_from_model)
                .collect()
        })
}

pub async fn create_generated_video_record<C>(
    db: &C,
    input: CreateGeneratedVideoRecordDraftInput,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::create_generated_video_record(db, input)
        .await
        .map(generated_video_record_from_model)
}

pub async fn create_generated_video_record_with_generation_run<C>(
    db: &C,
    input: CreateGeneratedVideoRecordDraftInput,
    generation_run: RecordStartedGenerationRunInput,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::create_generated_video_record_with_generation_run(db, input, generation_run)
        .await
        .map(generated_video_record_from_model)
}

pub async fn create_prompt_generated_video_record_with_generation_run<C>(
    db: &C,
    input: CreateGeneratedVideoRecordDraftInput,
    generation_run: RecordStartedGenerationRunInput,
    prompt_id: i64,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::create_prompt_generated_video_record_with_generation_run(
        db,
        input,
        generation_run,
        prompt_id,
    )
    .await
    .map(generated_video_record_from_model)
}

pub async fn update_generated_video_record<C: ConnectionTrait>(
    db: &C,
    video_id: i64,
    input: UpdateGeneratedVideoRecordInput,
) -> Result<GeneratedVideoRecord, VideoDbError> {
    super::update_generated_video_record(db, video_id, input)
        .await
        .map(generated_video_record_from_model)
}

pub async fn update_generated_video_waiting_status_and_prompt_record<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::update_generated_video_waiting_status_and_prompt_record(db, video_id, prompt_id, status)
        .await
        .map(generated_video_record_from_model)
}

pub async fn mark_generated_video_error_status_and_prompt_record<C>(
    db: &C,
    video_id: i64,
    prompt_id: Option<i64>,
    status: &str,
    message: &str,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::mark_generated_video_error_status_and_prompt_record(
        db, video_id, prompt_id, status, message,
    )
    .await
    .map(generated_video_record_from_model)
}

pub async fn mark_generated_videos_error_status_and_prompt_records(
    db: &impl crate::DbExecutor,
    inputs: &[GeneratedVideoErrorStatusBatchInput],
    message: &str,
) -> Result<Vec<i64>, VideoDbError> {
    super::mark_generated_videos_error_status_and_prompt_records(db, inputs, message).await
}

pub async fn mark_generated_video_job_and_prompt_record<C>(
    db: &C,
    video_id: i64,
    provider_job_id: &str,
    metadata_json: Option<Value>,
    prompt_id: Option<i64>,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::mark_generated_video_job_and_prompt_record(
        db,
        video_id,
        provider_job_id,
        metadata_json,
        prompt_id,
    )
    .await
    .map(generated_video_record_from_model)
}

pub async fn mark_generated_video_completed_and_link_prompt_record<C>(
    db: &C,
    video_id: i64,
    file_path: &str,
    thumbnail_path: Option<&str>,
    properties: &VideoFileProperties,
    metadata_json: Option<Value>,
    prompt_id: Option<i64>,
) -> Result<GeneratedVideoRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::mark_generated_video_completed_and_link_prompt_record(
        db,
        video_id,
        file_path,
        thumbnail_path,
        properties,
        metadata_json,
        prompt_id,
    )
    .await
    .map(generated_video_record_from_model)
}

pub fn chapter_video_prompt_from_model(
    model: a4n_db_models::models::chapter::chapter_video_prompt::Model,
) -> ChapterVideoPromptRecord {
    chapter_video_prompt_record_from_model(model)
}

pub async fn chapter_video_prompt_by_id(
    db: &impl crate::DbExecutor,
    prompt_id: i64,
) -> Result<Option<ChapterVideoPromptRecord>, VideoDbError> {
    super::chapter_video_prompt_by_id(db, prompt_id)
        .await
        .map(|prompt| prompt.map(chapter_video_prompt_record_from_model))
}

pub async fn chapter_video_prompts_for_batch_generation(
    db: &impl crate::DbExecutor,
    prompt_ids: &[i64],
) -> Result<Vec<ChapterVideoPromptRecord>, VideoDbError> {
    super::chapter_video_prompts_for_batch_generation(db, prompt_ids)
        .await
        .map(|prompts| {
            prompts
                .into_iter()
                .map(chapter_video_prompt_record_from_model)
                .collect()
        })
}

pub async fn chapter_video_prompts_for_chapter<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterVideoPromptRecord>, DbErr> {
    super::chapter_video_prompts_for_chapter(db, project_id, chapter_id)
        .await
        .map(chapter_video_prompt_records)
}

pub async fn sync_chapter_video_prompts_with_generation_runs<C>(
    db: &C,
    chapter_id: i64,
    drafts: Vec<ChapterVideoPromptSyncDraft>,
    policy: ChapterVideoPromptSyncPolicy,
) -> Result<Vec<ChapterVideoPromptRecord>, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::prompts::consistency::sync_chapter_video_prompts_with_generation_runs(
        db, chapter_id, drafts, policy,
    )
    .await
    .map(chapter_video_prompt_records)
}

pub async fn chapter_video_prompt_records_for_types<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_types: &[&str],
) -> Result<Vec<ChapterVideoPromptRecord>, DbErr> {
    super::chapter_video_prompt_records_for_types(db, project_id, chapter_id, prompt_types)
        .await
        .map(chapter_video_prompt_records)
}

pub async fn chapter_video_prompt_workflow_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    workflow_mode: &str,
    exclude_running: bool,
) -> Result<Vec<ChapterVideoPromptRecord>, DbErr> {
    super::chapter_video_prompt_workflow_records(
        db,
        project_id,
        chapter_id,
        prompt_type,
        workflow_mode,
        exclude_running,
    )
    .await
    .map(chapter_video_prompt_records)
}

pub async fn chapter_video_prompt_segment_plan_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_type: &str,
    only_plan_row: bool,
) -> Result<Vec<ChapterVideoPromptRecord>, DbErr> {
    super::chapter_video_prompt_segment_plan_records(
        db,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        only_plan_row,
    )
    .await
    .map(chapter_video_prompt_records)
}

pub async fn chapter_video_prompt_workflow_shot_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_type: &str,
    shot_number: i32,
) -> Result<Vec<ChapterVideoPromptRecord>, DbErr> {
    super::chapter_video_prompt_workflow_shot_records(
        db,
        project_id,
        chapter_id,
        workflow_mode,
        prompt_type,
        shot_number,
    )
    .await
    .map(chapter_video_prompt_records)
}

pub async fn update_chapter_video_prompt_record<C: ConnectionTrait>(
    db: &C,
    prompt: ChapterVideoPromptRecord,
    input: UpdateChapterVideoPromptRecordInput,
) -> Result<ChapterVideoPromptRecord, VideoDbError> {
    super::update_chapter_video_prompt_record(
        db,
        chapter_video_prompt_model_from_record(prompt),
        input,
    )
    .await
    .map(chapter_video_prompt_record_from_model)
}

pub async fn with_video_plan_source_sync<C: ConnectionTrait>(
    db: &C,
    prompt: ChapterVideoPromptRecord,
) -> Result<ChapterVideoPromptRecord, VideoDbError> {
    super::with_video_plan_source_sync(db, chapter_video_prompt_model_from_record(prompt))
        .await
        .map(chapter_video_prompt_record_from_model)
}

pub async fn update_chapter_video_prompt_with_policy<C>(
    db: &C,
    prompt_id: i64,
    input: ChapterVideoPromptPatch,
    allow_downstream: bool,
) -> Result<ChapterVideoPromptRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::prompts::consistency::update_chapter_video_prompt_with_policy(
        db,
        prompt_id,
        input,
        allow_downstream,
    )
    .await
    .map(chapter_video_prompt_record_from_model)
}

pub async fn update_chapter_video_segment_plan_metadata<C>(
    db: &C,
    prompt_id: i64,
    expected_updated_at: chrono::NaiveDateTime,
    metadata_json: Value,
) -> Result<ChapterVideoPromptRecord, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = super::prompts::consistency::update_chapter_video_segment_plan_metadata(
        db,
        prompt_id,
        expected_updated_at,
        metadata_json,
    )
    .await?;
    super::with_video_plan_source_sync(db, prompt)
        .await
        .map(chapter_video_prompt_record_from_model)
}

pub async fn update_chapter_video_prompt_with_generation_run_checkpoint<C>(
    db: &C,
    prompt_id: i64,
    input: ChapterVideoPromptPatch,
    metadata_json: Option<Value>,
    allow_downstream: bool,
    generation_run: RecordCompletedGenerationRunWithCheckpointInput,
) -> Result<ChapterVideoPromptRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    super::prompts::consistency::update_chapter_video_prompt_with_generation_run_checkpoint(
        db,
        prompt_id,
        input,
        metadata_json,
        allow_downstream,
        generation_run,
    )
    .await
    .map(chapter_video_prompt_record_from_model)
}

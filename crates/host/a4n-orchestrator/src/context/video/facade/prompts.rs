//! 视频提示词 facade。

use a4n_domain::video_prompt::ChapterVideoPromptRecord;

use super::*;

impl OrchestratorContext {
    pub(crate) async fn chapter_video_prompt(
        &self,
        prompt_id: i64,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        self.video
            .chapter_video_prompt_by_id(prompt_id)
            .await?
            .ok_or_else(|| AppError::NotFound("视频提示词不存在".to_string()))
    }

    pub(crate) async fn chapter_video_prompts_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
        self.video
            .chapter_video_prompts_for_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn chapter_video_prompt_timeline_records_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptTimelineRecord>, AppError> {
        self.video
            .chapter_video_prompt_timeline_records_for_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn chapter_video_prompt_video_binding_records_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<(i64, Option<i64>)>, AppError> {
        self.video
            .chapter_video_prompt_video_binding_records_for_chapter(project_id, chapter_id)
            .await
    }

    pub(crate) async fn chapter_video_prompts_for_batch_generation(
        &self,
        prompt_ids: &[i64],
    ) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
        self.video
            .chapter_video_prompts_for_batch_generation(prompt_ids)
            .await
    }

    pub(crate) async fn update_chapter_video_prompt_with_policy(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        allow_downstream: bool,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        self.video
            .update_chapter_video_prompt_with_policy(prompt_id, input, allow_downstream)
            .await
    }

    pub(crate) async fn update_chapter_video_prompt_with_policy_expected(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        allow_downstream: bool,
        expected_updated_at: &str,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        self.video
            .update_chapter_video_prompt_with_policy_expected(
                prompt_id,
                input,
                allow_downstream,
                expected_updated_at,
            )
            .await
    }

    pub(crate) async fn update_chapter_video_segment_plan_metadata(
        &self,
        prompt_id: i64,
        expected_updated_at: chrono::NaiveDateTime,
        metadata_json: Value,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        self.video
            .update_chapter_video_segment_plan_metadata(
                prompt_id,
                expected_updated_at,
                metadata_json,
            )
            .await
    }

    pub(crate) async fn update_chapter_video_segment_recipe(
        &self,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
        self.video
            .update_chapter_video_segment_recipe(prompt_id, recipe)
            .await
    }

    pub(crate) async fn update_chapter_video_prompt_with_generation_run_checkpoint_expected(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        metadata_json: Option<Value>,
        allow_downstream: bool,
        expected_updated_at: Option<&str>,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        self.video
            .update_chapter_video_prompt_with_generation_run_checkpoint_expected(
                prompt_id,
                input,
                metadata_json,
                allow_downstream,
                expected_updated_at,
                generation_run,
            )
            .await
    }

    pub(crate) async fn delete_chapter_video_prompts_checked(
        &self,
        prompt_ids: &[i64],
        allow_downstream: bool,
    ) -> Result<u64, AppError> {
        self.video
            .delete_chapter_video_prompts_with_policy(prompt_ids, allow_downstream)
            .await
    }

    pub(crate) async fn prompt_downstream_dependencies(
        &self,
        prompt_id: i64,
    ) -> Result<VideoPromptDownstreamDependencies, AppError> {
        self.video.prompt_downstream_dependencies(prompt_id).await
    }

    pub(crate) async fn ensure_frame_prompt_image_generation_allowed(
        &self,
        prompt_id: i64,
        generation_source: &str,
    ) -> Result<(), AppError> {
        self.video
            .ensure_frame_prompt_image_generation_allowed(prompt_id, generation_source)
            .await
    }

    pub(crate) async fn effective_frame_prompt_image_reference_ids(
        &self,
        prompt_id: i64,
        generation_source: &str,
        requested_reference_image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        self.video
            .effective_frame_prompt_image_reference_ids(
                prompt_id,
                generation_source,
                requested_reference_image_ids,
            )
            .await
    }

    pub(crate) async fn ensure_prompt_stage_mutable(
        &self,
        prompt: &ChapterVideoPromptRecord,
        operation: &str,
    ) -> Result<(), AppError> {
        self.video
            .ensure_chapter_video_prompt_stage_mutable(prompt, operation)
            .await
    }

    pub(crate) async fn ensure_prompt_stage_mutable_ignoring(
        &self,
        prompt: &ChapterVideoPromptRecord,
        operation: &str,
        ignored_prompt_ids: &BTreeSet<i64>,
    ) -> Result<(), AppError> {
        self.video
            .ensure_chapter_video_prompt_stage_mutable_ignoring(
                prompt,
                operation,
                ignored_prompt_ids,
            )
            .await
    }

    pub(crate) async fn ensure_prompt_stages_mutable(
        &self,
        prompts: &[ChapterVideoPromptRecord],
        operation: &str,
    ) -> Result<(), AppError> {
        self.video
            .ensure_chapter_video_prompt_stages_mutable(prompts, operation)
            .await
    }

    pub(crate) async fn ensure_prompt_stages_mutable_skipping_generated_images(
        &self,
        prompts: &[ChapterVideoPromptRecord],
        operation: &str,
    ) -> Result<(), AppError> {
        self.video
            .ensure_chapter_video_prompt_stages_mutable_skipping_generated_images(
                prompts, operation,
            )
            .await
    }
}

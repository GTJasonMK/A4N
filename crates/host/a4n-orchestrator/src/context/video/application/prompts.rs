//! 视频提示词与阶段门禁 owner。

use a4n_domain::video_prompt::ChapterVideoPromptRecord;

use super::*;

impl VideoApplication {
    pub(in crate::context) async fn chapter_video_prompt_by_id(
        &self,
        prompt_id: i64,
    ) -> Result<Option<ChapterVideoPromptRecord>, AppError> {
        a4n_db::video::read_models::chapter_video_prompt_by_id(
            self.database.connection(),
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn chapter_video_prompts_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
        a4n_db::video::read_models::chapter_video_prompts_for_chapter(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(AppError::Database)
    }

    pub(in crate::context) async fn chapter_video_prompt_timeline_records_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptTimelineRecord>, AppError> {
        a4n_db::video::chapter_video_prompt_timeline_records_for_chapter(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(AppError::Database)
    }

    pub(in crate::context) async fn chapter_video_prompt_video_binding_records_for_chapter(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<(i64, Option<i64>)>, AppError> {
        a4n_db::video::chapter_video_prompt_video_binding_records_for_chapter(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(AppError::Database)
    }

    pub(in crate::context) async fn chapter_video_prompts_for_batch_generation(
        &self,
        prompt_ids: &[i64],
    ) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
        a4n_db::video::read_models::chapter_video_prompts_for_batch_generation(
            self.database.connection(),
            prompt_ids,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn update_chapter_video_prompt_with_policy(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        allow_downstream: bool,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        a4n_db::video::read_models::update_chapter_video_prompt_with_policy(
            self.connection(),
            prompt_id,
            input,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn update_chapter_video_segment_plan_metadata(
        &self,
        prompt_id: i64,
        expected_updated_at: chrono::NaiveDateTime,
        metadata_json: Value,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        a4n_db::video::read_models::update_chapter_video_segment_plan_metadata(
            self.connection(),
            prompt_id,
            expected_updated_at,
            metadata_json,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn update_chapter_video_segment_recipe(
        &self,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, AppError> {
        a4n_db::video::prompts::consistency::update_chapter_video_segment_recipe(
            self.connection(),
            prompt_id,
            recipe,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn update_chapter_video_prompt_with_generation_run_checkpoint(
        &self,
        prompt_id: i64,
        input: ChapterVideoPromptPatch,
        metadata_json: Option<Value>,
        allow_downstream: bool,
        generation_run: RecordCompletedGenerationRunWithCheckpointInput,
    ) -> Result<ChapterVideoPromptRecord, AppError> {
        a4n_db::video::read_models::update_chapter_video_prompt_with_generation_run_checkpoint(
            self.connection(),
            prompt_id,
            input,
            metadata_json,
            allow_downstream,
            generation_run,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn delete_chapter_video_prompts_with_policy(
        &self,
        prompt_ids: &[i64],
        allow_downstream: bool,
    ) -> Result<u64, AppError> {
        a4n_db::video::prompts::consistency::delete_chapter_video_prompts_with_policy(
            self.connection(),
            prompt_ids,
            allow_downstream,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn prompt_downstream_dependencies(
        &self,
        prompt_id: i64,
    ) -> Result<VideoPromptDownstreamDependencies, AppError> {
        a4n_db::video::prompts::consistency::prompt_downstream_dependencies(
            self.connection(),
            prompt_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn ensure_frame_prompt_image_generation_allowed(
        &self,
        prompt_id: i64,
        generation_source: &str,
    ) -> Result<(), AppError> {
        a4n_db::video::prompts::consistency::ensure_frame_prompt_image_generation_allowed(
            self.connection(),
            prompt_id,
            generation_source,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn effective_frame_prompt_image_reference_ids(
        &self,
        prompt_id: i64,
        generation_source: &str,
        requested_reference_image_ids: &[i64],
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::video::prompts::consistency::effective_frame_prompt_image_reference_ids(
            self.connection(),
            prompt_id,
            generation_source,
            requested_reference_image_ids,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn ensure_chapter_video_prompt_stage_mutable(
        &self,
        prompt: &ChapterVideoPromptRecord,
        operation: &str,
    ) -> Result<(), AppError> {
        a4n_db::video::prompts::consistency::ensure_chapter_video_prompt_stage_mutable(
            self.connection(),
            prompt,
            operation,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn ensure_chapter_video_prompt_stage_mutable_ignoring(
        &self,
        prompt: &ChapterVideoPromptRecord,
        operation: &str,
        ignored_prompt_ids: &BTreeSet<i64>,
    ) -> Result<(), AppError> {
        a4n_db::video::prompts::consistency::ensure_chapter_video_prompt_stage_mutable_ignoring(
            self.connection(),
            prompt,
            operation,
            ignored_prompt_ids,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn ensure_chapter_video_prompt_stages_mutable(
        &self,
        prompts: &[ChapterVideoPromptRecord],
        operation: &str,
    ) -> Result<(), AppError> {
        a4n_db::video::prompts::consistency::ensure_chapter_video_prompt_stages_mutable(
            self.connection(),
            prompts,
            operation,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn ensure_chapter_video_prompt_stages_mutable_skipping_generated_images(
        &self,
        prompts: &[ChapterVideoPromptRecord],
        operation: &str,
    ) -> Result<(), AppError> {
        a4n_db::video::prompts::consistency::ensure_chapter_video_prompt_stages_mutable_skipping_generated_images(
            self.connection(),
            prompts,
            operation,
        )
        .await
        .map_err(video_db_error)
    }
}

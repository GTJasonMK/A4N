//! Generation task start validation and Host-facing start wrappers.

use super::fingerprints::{
    map_image_generation_start_error, map_optimize_generation_start_error,
    map_outline_generation_start_error, map_portrait_generation_start_error,
    map_video_generation_start_error,
};
use super::*;

impl OrchestratorContext {
    pub(crate) async fn start_manga_pipeline_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::manga::MangaPipelineStartInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let image_stage_requested = match input.run_stage {
            Some(a4n_domain::manga::MangaPipelineArtifactStage::Image) => true,
            Some(_) => false,
            None => matches!(
                input.stop_after,
                None | Some(a4n_api_contract::manga::MangaPipelineStopAfter::Image)
            ),
        };
        if input.image_size.is_none() && image_stage_requested {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "自动成图需要指定图片尺寸".into(),
            ));
        }
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        let source = self
            .chapter_manga_source_by_number(input.project_id, input.chapter_number, true)
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        let portraits = self
            .manga_character_portrait_refs(input.project_id)
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        let asset_project_ids = self
            .manga_reference_asset_project_ids(input.project_id)
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        let available_reference_images = self
            .video_reference_candidate_images(&asset_project_ids, source.chapter_id)
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        crate::manga::ensure_manga_source_can_write(&source, "启动漫画流水线")
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        let fingerprint = serde_json::json!({
            "pipeline_contract": "manga_page_pipeline_v2",
            "source": crate::manga::manga_pipeline_run_fingerprint(
                &source.content,
                &portraits,
                &available_reference_images,
                input.run_stage,
            ),
            "image_size": input.image_size,
            "stop_after": input.stop_after,
        });
        self.generation
            .start_manga_pipeline_task(user_id, input, source.chapter_id, fingerprint)
            .await
    }

    pub(crate) async fn start_shooting_script_generation_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::shooting_script::StartChapterShootingScriptGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        crate::shooting_script::validate_shooting_generation_mode(input.mode, &input.shot_ids)
            .map_err(GenerationTaskAccessError::Project)?;
        let (workflow, _) = self
            .prepare_shooting_script_workflow_input(input, None)
            .await
            .map_err(GenerationTaskAccessError::Project)?;
        let fingerprint = serde_json::json!({
            "source": workflow.input_fingerprint_json,
            "mode": input.mode,
            "model": self.llm_model_name(),
        });
        self.generation
            .start_shooting_script_generation_task(user_id, input, fingerprint)
            .await
    }

    pub(crate) async fn start_chapter_generation_task(
        &self,
        user_id: i64,
        input: &StartChapterGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_chapter_generation_application_access(
            user_id,
            input.project_id,
            input.chapter_number,
            "生成章节正文",
        )
        .await
        .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.validate_chapter_generation_plan_input(
            input.project_id,
            input.chapter_number,
            input.writing_notes.as_deref(),
        )
        .await
        .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.validate_chapter_generation_client_rag_input(input)
            .await
            .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.generation
            .start_chapter_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_inspiration_generation_task(
        &self,
        user_id: i64,
        input: &StartInspirationGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        crate::inspiration::required_message_content(&input.message)
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        self.verify_project_permission(
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(GenerationTaskAccessError::Project)?;
        self.ensure_text_source_project(input.project_id, "灵感对话")
            .await
            .map_err(GenerationTaskAccessError::InvalidArgument)?;
        self.generation
            .start_inspiration_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_visual_plan_prompt_generation_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::chapter_visual_plan::StartVisualPlanPromptGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let (plan, _) = self
            .visual_plan_generation_source(
                user_id,
                input.project_id,
                input.source_chapter_id,
                input.record_id,
                input.expected_revision,
            )
            .await
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?;
        if !plan
            .plan
            .segments
            .iter()
            .any(|segment| segment.id == input.segment_id)
            || input.user_description.chars().count() > 100_000
        {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "镜头段不存在或生成描述超过长度限制".into(),
            ));
        }
        self.generation
            .start_visual_plan_prompt_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_outline_generation_task(
        &self,
        user_id: i64,
        input: &StartOutlineGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_outline_generation_start(user_id, input)
            .await
            .map_err(map_outline_generation_start_error)?;
        self.generation
            .start_outline_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_portrait_generation_task(
        &self,
        user_id: i64,
        input: &StartPortraitGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_portrait_generation_start(user_id, input)
            .await
            .map_err(map_portrait_generation_start_error)?;
        self.generation
            .start_portrait_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_image_generation_task(
        &self,
        user_id: i64,
        input: &StartImageGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_image_generation_start(user_id, input)
            .await
            .map_err(map_image_generation_start_error)?;
        self.generation
            .start_image_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_video_generation_task(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.validate_video_generation_start(user_id, input)
            .await
            .map_err(map_video_generation_start_error)?;
        self.generation
            .start_video_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_blueprint_generation_task(
        &self,
        user_id: i64,
        input: &StartBlueprintGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        if input.title.trim().is_empty() || input.genre.trim().is_empty() {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "蓝图标题和题材不能为空".to_string(),
            ));
        }
        if input.target_length <= 0 {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "蓝图目标章节数必须大于 0".to_string(),
            ));
        }
        self.verify_project_permission(
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(GenerationTaskAccessError::Project)?;
        self.generation
            .start_blueprint_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn start_blueprint_refinement_task(
        &self,
        user_id: i64,
        input: &StartBlueprintRefinementInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map_err(GenerationTaskAccessError::Project)?;
        if self
            .novel_blueprint_for_project(input.project_id)
            .await
            .map_err(|error| GenerationTaskAccessError::InvalidArgument(error.to_string()))?
            .is_none()
        {
            return Err(GenerationTaskAccessError::InvalidArgument(
                "蓝图不存在".to_string(),
            ));
        }
        self.generation
            .start_blueprint_refinement_task(user_id, input)
            .await
    }

    pub(crate) async fn start_optimize_chat_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeChatGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let session = self
            .validated_optimize_chat_generation_session(
                user_id,
                input.request.session_id,
                &input.request.message,
            )
            .await
            .map_err(map_optimize_generation_start_error)?;
        self.generation
            .start_optimize_chat_generation_task(user_id, input, session.project_id)
            .await
    }

    pub(crate) async fn start_optimize_analysis_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizeAnalysisGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let session = self
            .optimize_session_for_user(user_id, input.session_id)
            .await
            .map_err(map_optimize_generation_start_error)?;
        self.generation
            .start_optimize_analysis_generation_task(user_id, input, session.project_id)
            .await
    }

    pub(crate) async fn start_optimize_patch_generation_task(
        &self,
        user_id: i64,
        input: &StartOptimizePatchGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        let session = self
            .optimize_session_for_user(user_id, input.request.session_id)
            .await
            .map_err(map_optimize_generation_start_error)?;
        self.generation
            .start_optimize_patch_generation_task(user_id, input, session.project_id)
            .await
    }
}

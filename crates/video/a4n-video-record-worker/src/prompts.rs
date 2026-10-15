use super::*;
use a4n_api_contract::video::ChapterVideoPromptSummary;
use a4n_video_record_worker_contract::{
    ChapterVideosInput, LoadVideoSegmentPlanDraftInput, UpdateVideoSegmentPlanDraftRecipeInput,
    VIDEO_PROMPT_DRAFT_LOAD_OPERATION, VIDEO_PROMPT_DRAFT_UPDATE_OPERATION,
    VIDEO_PROMPT_LIST_OPERATION, VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION,
    VideoSegmentPlanDraftSummary, VideoSourceInput,
};
use a4n_worker_contract::WorkerEnvelope;
use serde_json::Value;

impl WorkerState {
    pub(super) async fn load_video_segment_plan_draft(
        &self,
        request: WorkerEnvelope<LoadVideoSegmentPlanDraftInput>,
    ) -> Result<VideoSegmentPlanDraftSummary, WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_DRAFT_LOAD_OPERATION)?;
        validate_required_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        let input = request.payload;
        WorkerFailure::require_positive_id(input.project_id, "project_id")?;
        WorkerFailure::require_positive_id(input.chapter_id, "chapter_id")?;
        WorkerFailure::require_positive_id(input.workflow_run_id, "workflow_run_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let run = a4n_db::workflow::workflow_run_summary_by_id(
            self.database.connection(),
            input.workflow_run_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_workflow_read_error_as_invalid_request)?;
        if run.workflow_kind != "video_segment_plan"
            || run.project_id != Some(input.project_id)
            || run.chapter_id != Some(input.chapter_id)
            || run.mode.as_deref()
                != Some(a4n_domain::video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(WorkerFailure::invalid_request(format!(
                "片段规划 workflow {} 与当前项目或章节不一致",
                input.workflow_run_id
            )));
        }
        if !a4n_db::workflow::workflow_run_is_running_or_resumable(&run.status, run.can_resume) {
            return Err(WorkerFailure::conflict(format!(
                "{}:加载片段规划草稿执行方案",
                run.status
            )));
        }
        let fingerprint = run
            .input_fingerprint_json
            .as_ref()
            .and_then(Value::as_object)
            .ok_or_else(|| WorkerFailure::conflict("缺少片段规划 workflow 输入指纹"))?;
        if fingerprint.get("schema_version").and_then(Value::as_i64) != Some(1)
            || fingerprint.get("workflow_kind").and_then(Value::as_str)
                != Some("video_segment_plan")
            || fingerprint.get("project_id").and_then(Value::as_i64) != Some(input.project_id)
            || fingerprint.get("chapter_id").and_then(Value::as_i64) != Some(input.chapter_id)
            || fingerprint.get("workflow_mode").and_then(Value::as_str)
                != Some(a4n_domain::video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(WorkerFailure::conflict("片段规划 workflow 输入指纹无效"));
        }
        let style = fingerprint
            .get("options")
            .and_then(Value::as_object)
            .and_then(|options| options.get("style"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| WorkerFailure::conflict("片段规划 workflow 缺少 style"))?;
        let checkpoints = a4n_db::video::video_segment_plan_draft_checkpoints(
            self.database.connection(),
            run.id,
            "video_segment_plan.shot.",
        )
        .await
        .map_err(video_error)?
        .into_iter()
        .map(
            |checkpoint| a4n_video_contract::VideoSegmentPlanDraftCheckpoint {
                step_key: checkpoint.step_key,
                extra_json: checkpoint.extra_json,
                parsed_output_json: checkpoint.parsed_output_json,
                artifacts_json: checkpoint.artifacts_json,
            },
        )
        .collect::<Vec<_>>();
        let (segments, recipes, completed_shot_count, total_shot_count) = if checkpoints.is_empty()
        {
            (Vec::new(), Vec::new(), 0, 0)
        } else {
            a4n_video_contract::load_video_segment_plan_draft_recipe_state(&checkpoints, style)
                .map_err(a4n_worker_db_adapter::map_app_error)?
        };
        Ok(VideoSegmentPlanDraftSummary {
            run: a4n_workflow_core::workflow_run_summary_to_api(run),
            segments,
            recipes,
            completed_shot_count,
            total_shot_count,
        })
    }

    pub(super) async fn update_video_segment_plan_draft_recipe(
        &self,
        request: WorkerEnvelope<UpdateVideoSegmentPlanDraftRecipeInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_DRAFT_UPDATE_OPERATION)?;
        let input = request.payload.clone();
        let load_request = WorkerEnvelope {
            schema_version: request.schema_version,
            request_id: request.request_id.clone(),
            trace_id: request.trace_id.clone(),
            principal: request.principal.clone(),
            operation_id: VIDEO_PROMPT_DRAFT_LOAD_OPERATION.to_string(),
            idempotency_key: request.idempotency_key.clone(),
            project_id: request.project_id,
            root_project_id: request.root_project_id,
            target_revision: request.target_revision,
            runtime_config_snapshot_ref: request.runtime_config_snapshot_ref.clone(),
            lease: request.lease.clone(),
            payload: LoadVideoSegmentPlanDraftInput {
                project_id: input.project_id,
                chapter_id: input.chapter_id,
                workflow_run_id: input.workflow_run_id,
            },
        };
        let summary = self.load_video_segment_plan_draft(load_request).await?;
        let style = summary
            .run
            .input_fingerprint_json
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|fingerprint| fingerprint.get("options"))
            .and_then(Value::as_object)
            .and_then(|options| options.get("style"))
            .and_then(Value::as_str)
            .filter(|style| !style.trim().is_empty())
            .ok_or_else(|| WorkerFailure::conflict("片段规划 workflow 缺少 style"))?;
        a4n_workflow_core::update_video_segment_plan_draft_recipe_checkpoint(
            self.database.connection(),
            input.project_id,
            input.workflow_run_id,
            &input.source_shot_id,
            input.segment_index_in_shot,
            style,
            input.recipe,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_app_error)
    }

    pub(super) async fn list_prompts(
        &self,
        request: WorkerEnvelope<ChapterVideosInput>,
    ) -> Result<Vec<ChapterVideoPromptSummary>, WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_LIST_OPERATION)?;
        validate_required_project_scope(&request, request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        let input = request.payload;
        WorkerFailure::require_positive_id(input.project_id, "project_id")?;
        WorkerFailure::require_positive_id(input.chapter_id, "chapter_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        validate_video_source(
            &self.database,
            input.project_id,
            input.chapter_id,
            "查看视频提示词",
            false,
        )
        .await?;
        let prompts = a4n_db::video::read_models::chapter_video_prompts_for_chapter(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_database_error)?;
        let mut result = Vec::with_capacity(prompts.len());
        for prompt in prompts {
            let prompt = a4n_db::video::read_models::with_video_plan_source_sync(
                self.database.connection(),
                prompt,
            )
            .await
            .map_err(video_error)?;
            result.push(ChapterVideoPromptSummary::from(&prompt));
        }
        Ok(result)
    }

    pub(super) async fn validate_source(
        &self,
        request: WorkerEnvelope<VideoSourceInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION)?;
        validate_required_project_scope(&request, request.payload.project_id)?;
        let input = request.payload;
        WorkerFailure::require_positive_id(input.project_id, "project_id")?;
        WorkerFailure::require_positive_id(input.chapter_id, "chapter_id")?;
        validate_video_source(
            &self.database,
            input.project_id,
            input.chapter_id,
            &input.operation,
            input.writable,
        )
        .await
        .map(|_| ())
    }
}

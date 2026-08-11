use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, MangaPipelineStatusSummary, OptimizeMangaPromptInput,
    UpdateMangaPromptInput,
};
use a4n_core::{error::AppError, models::chapter::chapter_manga_prompt};
use a4n_domain::DownstreamConflictStrategy;
use a4n_prompt::rendering::PromptRenderer;

use super::record_errors::manga_pipeline_db_error;
use super::{
    MangaGeneratePagePromptsInput, MangaPipelineRequestPreviewInput,
    MangaPipelineRequestPreviewStep, MangaPromptDownstreamCleanupPort,
    MangaPromptOptimizationInput, ensure_manga_source_can_write, manga_pipeline_fingerprint,
    resolve_manga_prompt_downstream_conflict,
};
use crate::workflow::{
    ListWorkflowRunsInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING,
};
use crate::{
    OrchestratorContext, collaboration::ProjectAccessRole, project::required_text,
    shooting_script::ChapterShootingScriptSourceInput,
};

pub struct MangaPipelineApplicationInput<'a> {
    pub project_id: i64,
    pub chapter_number: i32,
    pub task_id: &'a str,
    pub resume_from_checkpoint: bool,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

pub struct MangaPipelinePreviewApplicationInput<'a> {
    pub project_id: i64,
    pub chapter_number: i32,
    pub resume_from_checkpoint: bool,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

impl OrchestratorContext {
    pub async fn preflight_manga_pipeline_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError> {
        let source = self
            .chapter_manga_source_by_number(project_id, chapter_number, true)
            .await?;
        ensure_manga_source_can_write(&source, "启动漫画流水线")?;
        self.ensure_media_workflow_v2_ready(project_id).await?;
        self.require_current_chapter_shooting_script(ChapterShootingScriptSourceInput {
            project_id,
            source_chapter_id: source.chapter_id,
        })
        .await?;
        Ok(())
    }

    pub async fn run_manga_pipeline_validated(
        &self,
        input: MangaPipelineApplicationInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let source = self
            .chapter_manga_source_by_number(input.project_id, input.chapter_number, true)
            .await?;
        ensure_manga_source_can_write(&source, "启动漫画流水线")?;
        let reference_context = self.manga_reference_context(input.project_id).await?;
        let workflow = self
            .begin_manga_pipeline_workflow_validated(
                input.project_id,
                source.chapter_id,
                &source.content,
                &reference_context,
                input.task_id.to_string(),
                input.resume_from_checkpoint,
            )
            .await?;
        let result = self
            .generate_page_prompts(MangaGeneratePagePromptsInput {
                chapter_id: source.chapter_id,
                project_id: input.project_id,
                prompt_renderer: input.prompt_renderer,
                reference_context,
                task_id: input.task_id,
                workflow_run_id: workflow.id,
                resume_from_checkpoint: input.resume_from_checkpoint,
            })
            .await;
        self.finalize_workflow_outcome_result(workflow.id, result, "漫画流水线")
            .await
    }

    pub async fn prepare_manga_pipeline_preview_validated(
        &self,
        input: MangaPipelinePreviewApplicationInput<'_>,
    ) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
        let source = self
            .chapter_manga_source_by_number(input.project_id, input.chapter_number, true)
            .await?;
        ensure_manga_source_can_write(&source, "预览漫画流水线提示词")?;
        let reference_context = self.manga_reference_context(input.project_id).await?;
        let workflow_fingerprint = manga_pipeline_fingerprint(&source.content, &reference_context);
        let workflow_run_id = if input.resume_from_checkpoint {
            Some(
                self.resumable_manga_workflow_validated(
                    input.project_id,
                    source.chapter_id,
                    &workflow_fingerprint,
                )
                .await?
                .id,
            )
        } else {
            None
        };
        self.preview_generate_page_prompt_requests(MangaPipelineRequestPreviewInput {
            chapter_id: source.chapter_id,
            project_id: input.project_id,
            prompt_renderer: input.prompt_renderer,
            reference_context,
            workflow_run_id,
            resume_from_checkpoint: input.resume_from_checkpoint,
        })
        .await
    }

    pub async fn list_manga_page_prompts_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        self.verify_manga_permission(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        self.manga_page_prompts_by_chapter(project_id, chapter_id)
            .await
    }

    pub async fn bind_manga_reference_image_validated(
        &self,
        user_id: i64,
        input: BindMangaReferenceImageInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let character_name =
            required_text(&input.character_name, "角色名称").map_err(AppError::InvalidArgument)?;
        let page_prompt = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, page_prompt.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(page_prompt.chapter_id, page_prompt.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "绑定漫画参考图")?;
        let downstream_image_id = resolve_manga_prompt_downstream_conflict(
            &page_prompt,
            input.downstream_strategy,
            "绑定参考图",
        )
        .map_err(AppError::InvalidArgument)?;
        let bound_asset = self
            .manga_reference_bound_asset(page_prompt.project_id, input.asset_kind, input.asset_id)
            .await?;
        let layout = page_prompt.layout.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!("漫画页面提示词 #{} 缺少 layout", page_prompt.id))
        })?;
        let layout = super::bind_reference_asset_in_layout(layout, character_name, bound_asset)
            .map_err(AppError::InvalidArgument)?;
        let updated = self
            .update_manga_page_prompt_layout(page_prompt, layout)
            .await?;
        if let Some(image_id) = downstream_image_id {
            cleanup.delete_downstream_images(&[image_id]).await?;
        }
        Ok(updated)
    }

    pub async fn update_manga_prompt_validated(
        &self,
        user_id: i64,
        input: UpdateMangaPromptInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let image_prompt = required_text(&input.image_prompt, "整页漫画提示词")
            .map_err(AppError::InvalidArgument)?;
        let page_prompt = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, page_prompt.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(page_prompt.chapter_id, page_prompt.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "修改整页漫画提示词")?;
        if page_prompt.image_prompt.trim() == image_prompt {
            return Ok(page_prompt);
        }
        let downstream_image_id = resolve_manga_prompt_downstream_conflict(
            &page_prompt,
            input.downstream_strategy,
            "修改提示词",
        )
        .map_err(AppError::InvalidArgument)?;
        let updated = self
            .update_manga_page_prompt_image_prompt(page_prompt, image_prompt.to_string())
            .await?;
        if let Some(image_id) = downstream_image_id {
            cleanup.delete_downstream_images(&[image_id]).await?;
        }
        Ok(updated)
    }

    pub async fn delete_manga_by_chapter_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        self.verify_manga_permission(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(chapter_id, project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "删除漫画产物")?;
        self.delete_manga_prompts_by_chapter(project_id, chapter_id)
            .await
            .map(|_| ())
    }

    pub async fn delete_manga_prompt_validated(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<(), AppError> {
        let page_prompt = self.required_manga_page_prompt(prompt_id).await?;
        self.verify_manga_permission(user_id, page_prompt.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(page_prompt.chapter_id, page_prompt.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "删除漫画提示词")?;
        let downstream_image_id = resolve_manga_prompt_downstream_conflict(
            &page_prompt,
            downstream_strategy,
            "删除提示词",
        )
        .map_err(AppError::InvalidArgument)?;
        if let Some(image_id) = downstream_image_id {
            cleanup.delete_downstream_images(&[image_id]).await?;
        }
        self.delete_manga_prompt_record(prompt_id).await
    }

    pub async fn get_manga_pipeline_status_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<MangaPipelineStatusSummary, AppError> {
        self.verify_manga_permission(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        let page_prompts = self
            .manga_page_prompts_by_chapter(project_id, chapter_id)
            .await?;
        let mut summary = MangaPipelineStatusSummary {
            chapter_id,
            pending: 0,
            generating: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            pipeline_status: None,
            stage: None,
            current: None,
            total: None,
            message: None,
            task_id: None,
            can_cancel: false,
            can_resume: false,
            has_checkpoint: false,
            checkpoint_valid: false,
        };
        for prompt in &page_prompts {
            match prompt.status.as_str() {
                "pending" => summary.pending += 1,
                "generating" => summary.generating += 1,
                "completed" => summary.completed += 1,
                "failed" => summary.failed += 1,
                "cancelled" => summary.cancelled += 1,
                status => {
                    return Err(AppError::InvalidArgument(format!(
                        "漫画页面提示词 #{} 包含未知状态: {status}",
                        prompt.id
                    )));
                }
            }
        }
        let run = self
            .workflow_runs(ListWorkflowRunsInput {
                project_id: Some(project_id),
                chapter_id: Some(chapter_id),
                workflow_kind: Some("manga_pipeline".to_string()),
                status: None,
                resumable_only: None,
                limit: Some(1),
            })
            .await?
            .into_iter()
            .next();
        if let Some(run) = run {
            summary.pipeline_status = Some(run.status.clone());
            summary.stage = run.current_step_key.clone();
            summary.task_id = run.active_task_id.clone();
            summary.can_cancel = run.status == WORKFLOW_STATUS_RUNNING;
            summary.message = run.error_message.clone();
            if let Some(progress) = run.progress.as_ref() {
                summary.current = Some(i32::try_from(progress.current).map_err(|_| {
                    AppError::InvalidArgument("漫画流水线 current 超出 i32 范围".to_string())
                })?);
                summary.total = Some(i32::try_from(progress.total).map_err(|_| {
                    AppError::InvalidArgument("漫画流水线 total 超出 i32 范围".to_string())
                })?);
                if summary.message.is_none() {
                    summary.message = Some(progress.message.clone());
                }
            }
            summary.has_checkpoint = self
                .workflow_checkpoint_models(run.id)
                .await?
                .iter()
                .any(|checkpoint| checkpoint.status == WORKFLOW_STATUS_COMPLETED);
            if summary.has_checkpoint {
                let source = self
                    .chapter_manga_source_by_id(chapter_id, project_id, false)
                    .await?;
                let reference_context = self.manga_reference_context(project_id).await?;
                let current_fingerprint =
                    manga_pipeline_fingerprint(&source.content, &reference_context);
                summary.checkpoint_valid =
                    run.input_fingerprint_json.as_ref() == Some(&current_fingerprint);
            }
            summary.can_resume = run.can_resume
                && summary.checkpoint_valid
                && matches!(
                    run.status.as_str(),
                    WORKFLOW_STATUS_FAILED
                        | WORKFLOW_STATUS_CANCELLED
                        | WORKFLOW_STATUS_INTERRUPTED
                );
        }
        Ok(summary)
    }

    pub async fn cancel_manga_pipeline_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<i64, AppError> {
        self.verify_manga_permission(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(chapter_id, project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "取消漫画流水线")?;
        let run = self
            .workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project_id),
                chapter_id: Some(chapter_id),
                workflow_kind: Some("manga_pipeline".to_string()),
                status: Some(WORKFLOW_STATUS_RUNNING.to_string()),
                resumable_only: None,
                limit: Some(1),
            })
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::InvalidState {
                current: "no_running_manga_pipeline".to_string(),
                operation: "取消漫画流水线".to_string(),
            })?;
        self.cancel_workflow_run_status_validated(run.id, Some("用户请求取消漫画流水线"))
            .await?;
        a4n_db::manga::cancel_manga_page_prompts(self.database.connection(), project_id, chapter_id)
            .await
            .map_err(manga_pipeline_db_error)
    }

    pub async fn optimize_manga_prompt_record_validated(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let page_prompt = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, page_prompt.project_id, ProjectAccessRole::Editor)
            .await?;
        let downstream_image_id = resolve_manga_prompt_downstream_conflict(
            &page_prompt,
            input.downstream_strategy,
            "优化提示词",
        )
        .map_err(AppError::InvalidArgument)?;
        self.optimize_manga_prompt_validated(
            MangaPromptOptimizationInput {
                page_prompt,
                instruction: input.instruction,
                downstream_image_ids: downstream_image_id.into_iter().collect(),
            },
            prompt_renderer,
            cleanup,
        )
        .await
    }

    async fn required_manga_page_prompt(
        &self,
        prompt_id: i64,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        if prompt_id <= 0 {
            return Err(AppError::InvalidArgument(
                "漫画提示词 ID 必须大于 0".to_string(),
            ));
        }
        let page_prompt = self
            .manga_page_prompt_by_id(prompt_id)
            .await?
            .ok_or_else(|| AppError::NotFound("整页漫画提示词不存在".to_string()))?;
        Ok(page_prompt)
    }

    async fn verify_manga_permission(
        &self,
        user_id: i64,
        project_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        if self.project_kind(project_id).await? == "text" {
            self.verify_project_permission(project_id, user_id, required)
                .await?;
            return Ok(());
        }
        let family = self.derived_project_family_context(project_id).await?;
        self.verify_project_permission(family.project.id, user_id, required)
            .await?;
        self.verify_project_permission(family.root_project.id, user_id, required)
            .await?;
        Ok(())
    }
}

use std::collections::HashMap;

use a4n_core::{
    dependency_conflict::DownstreamConflictStrategy, error::AppError,
    models::chapter::chapter_manga_prompt,
};
use a4n_prompt::rendering::PromptRenderer;
use serde::{Deserialize, Serialize};

use crate::workflow::WorkflowRunFinalizeOutcome;
use crate::{OrchestratorContext, collaboration::ProjectAccessRole, project::required_text};

use super::{
    MANGA_PAGE_PROMPT_TYPE, MangaGeneratePanelsInput, MangaPipelineRequestPreviewInput,
    MangaPipelineRequestPreviewStep, MangaPipelineStartStage, MangaPromptDownstreamCleanupPort,
    MangaPromptOptimizationInput, PIPELINE_STATUS_PROMPT_TYPE, ensure_manga_source_can_write,
    manga_checkpoint_matches_source, manga_pipeline_business_key, manga_pipeline_fingerprint,
    optional_layout_string, required_layout_bool, required_layout_i32, required_layout_string,
    resolve_manga_prompt_downstream_conflict,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindReferenceImageInput {
    pub prompt_id: i64,
    pub slot_id: String,
    pub asset_kind: String,
    pub asset_id: i64,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateMangaPromptInput {
    pub prompt_id: i64,
    pub image_prompt: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OptimizeMangaPromptRecordInput {
    pub prompt_id: i64,
    pub instruction: String,
    #[serde(default)]
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize)]
pub struct MangaPipelineStatusSummary {
    pub chapter_id: i64,
    pub pending: i64,
    pub generating: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub pipeline_status: Option<String>,
    pub stage: Option<String>,
    pub current: Option<i32>,
    pub total: Option<i32>,
    pub message: Option<String>,
    pub task_id: Option<String>,
    pub can_cancel: bool,
    pub can_resume: bool,
    pub has_checkpoint: bool,
    pub checkpoint_valid: bool,
}

pub struct MangaPipelineApplicationInput<'a> {
    pub project_id: i64,
    pub chapter_number: i32,
    pub task_id: &'a str,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<MangaPipelineStartStage>,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

pub struct MangaPipelinePreviewApplicationInput<'a> {
    pub project_id: i64,
    pub chapter_number: i32,
    pub resume_from_checkpoint: bool,
    pub start_from_stage: Option<MangaPipelineStartStage>,
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
        ensure_manga_source_can_write(&source, "启动漫画流水线")
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
                input.start_from_stage,
                Some(input.task_id.to_string()),
            )
            .await?;
        let variables = HashMap::new();
        let result = self
            .generate_panels(MangaGeneratePanelsInput {
                chapter_id: source.chapter_id,
                project_id: input.project_id,
                prompt_renderer: input.prompt_renderer,
                variables: &variables,
                reference_context,
                task_id: Some(input.task_id),
                workflow_run_id: Some(workflow.id),
                resume_from_checkpoint: input.resume_from_checkpoint,
                start_from_stage: input.start_from_stage,
            })
            .await;
        let outcome = match &result {
            Ok(_) => WorkflowRunFinalizeOutcome::Completed,
            Err(AppError::Cancelled) => WorkflowRunFinalizeOutcome::Cancelled,
            Err(_) => WorkflowRunFinalizeOutcome::Failed,
        };
        let error_message = result.as_ref().err().map(ToString::to_string);
        let finalized = self
            .finalize_workflow_run_outcome_validated(workflow.id, outcome, error_message.as_deref())
            .await;
        match (result, finalized) {
            (Ok(panels), Ok(_)) => Ok(panels),
            (Ok(_), Err(status_error)) => Err(status_error),
            (Err(error), Ok(_)) => Err(error),
            (Err(error), Err(status_error)) => Err(AppError::InvalidState {
                current: error.to_string(),
                operation: format!("漫画流水线失败，且 workflow 终态写入失败: {status_error}"),
            }),
        }
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
        let workflow_run_id = self
            .latest_workflow_run_by_business_key(&manga_pipeline_business_key(
                input.project_id,
                source.chapter_id,
            ))
            .await?
            .filter(|run| run.input_fingerprint_json.as_ref() == Some(&workflow_fingerprint))
            .map(|run| run.id);
        let variables = HashMap::new();
        self.preview_generate_panels_requests(MangaPipelineRequestPreviewInput {
            chapter_id: source.chapter_id,
            project_id: input.project_id,
            prompt_renderer: input.prompt_renderer,
            variables: &variables,
            reference_context,
            workflow_run_id,
            resume_from_checkpoint: input.resume_from_checkpoint,
            start_from_stage: input.start_from_stage,
        })
        .await
    }

    pub async fn list_manga_panels_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        self.verify_manga_permission(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        self.manga_page_prompts_by_chapter(project_id, chapter_id, MANGA_PAGE_PROMPT_TYPE)
            .await
    }

    pub async fn bind_manga_reference_image_validated(
        &self,
        user_id: i64,
        input: BindReferenceImageInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let slot_id =
            required_text(&input.slot_id, "参考图槽位").map_err(AppError::InvalidArgument)?;
        let asset_kind = required_text(&input.asset_kind, "参考图素材类型")
            .map_err(AppError::InvalidArgument)?;
        let panel = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, panel.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(panel.chapter_id, panel.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "绑定漫画参考图")?;
        let downstream_image_id = resolve_manga_prompt_downstream_conflict(
            &panel,
            input.downstream_strategy,
            "绑定参考图",
        )
        .map_err(AppError::InvalidArgument)?;
        let bound_asset = self
            .manga_reference_bound_asset(panel.project_id, asset_kind, input.asset_id)
            .await?;
        let layout =
            super::bind_reference_asset_in_layout(panel.layout.clone(), slot_id, bound_asset)
                .map_err(AppError::InvalidArgument)?;
        let updated = self.update_manga_page_prompt_layout(panel, layout).await?;
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
        let panel = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, panel.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(panel.chapter_id, panel.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "修改整页漫画提示词")?;
        if panel.image_prompt.trim() == image_prompt {
            return Ok(panel);
        }
        let downstream_image_id = resolve_manga_prompt_downstream_conflict(
            &panel,
            input.downstream_strategy,
            "修改提示词",
        )
        .map_err(AppError::InvalidArgument)?;
        let updated = self
            .update_manga_page_prompt_image_prompt(panel, image_prompt.to_string())
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
        let panel = self.required_manga_page_prompt(prompt_id).await?;
        self.verify_manga_permission(user_id, panel.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(panel.chapter_id, panel.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "删除漫画提示词")?;
        let downstream_image_id =
            resolve_manga_prompt_downstream_conflict(&panel, downstream_strategy, "删除提示词")
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
        let rows = self
            .manga_pipeline_status_rows(
                project_id,
                chapter_id,
                MANGA_PAGE_PROMPT_TYPE,
                PIPELINE_STATUS_PROMPT_TYPE,
            )
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
        for prompt in &rows.page_prompts {
            match prompt.status.as_str() {
                "pending" => summary.pending += 1,
                "generating" => summary.generating += 1,
                "completed" => summary.completed += 1,
                "failed" => summary.failed += 1,
                "cancelled" => summary.cancelled += 1,
                _ => {}
            }
        }
        let layout = match rows.status_row.as_ref() {
            Some(row) => Some(row.layout.clone().ok_or_else(|| {
                AppError::InvalidArgument(format!("漫画流水线状态 #{} 缺少 layout", row.id))
            })?),
            None => None,
        };
        if let Some(layout) = layout.as_ref() {
            summary.pipeline_status = Some(
                required_layout_string(layout, "pipeline_status")
                    .map_err(AppError::InvalidArgument)?,
            );
            summary.stage =
                Some(required_layout_string(layout, "stage").map_err(AppError::InvalidArgument)?);
            summary.current =
                Some(required_layout_i32(layout, "current").map_err(AppError::InvalidArgument)?);
            summary.total =
                Some(required_layout_i32(layout, "total").map_err(AppError::InvalidArgument)?);
            summary.message =
                Some(required_layout_string(layout, "message").map_err(AppError::InvalidArgument)?);
            summary.task_id =
                optional_layout_string(layout, "task_id").map_err(AppError::InvalidArgument)?;
            summary.has_checkpoint = match layout.get("checkpoint_data") {
                Some(value) => value
                    .as_object()
                    .map(|map| !map.is_empty())
                    .ok_or_else(|| {
                        AppError::InvalidArgument(
                            "漫画流水线 checkpoint_data 必须是对象".to_string(),
                        )
                    })?,
                None => false,
            };
            summary.can_cancel =
                required_layout_bool(layout, "can_cancel").map_err(AppError::InvalidArgument)?;
            summary.can_resume =
                required_layout_bool(layout, "can_resume").map_err(AppError::InvalidArgument)?;
            if summary.has_checkpoint {
                let source = self
                    .chapter_manga_source_by_id(chapter_id, project_id, false)
                    .await?;
                summary.checkpoint_valid = manga_checkpoint_matches_source(layout, &source)?;
                summary.can_resume &= summary.checkpoint_valid;
            }
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
        self.cancel_manga_pipeline_records(project_id, chapter_id)
            .await
    }

    pub async fn optimize_manga_prompt_record_validated(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptRecordInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<chapter_manga_prompt::Model, AppError> {
        let panel = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, panel.project_id, ProjectAccessRole::Editor)
            .await?;
        let downstream_image_id = resolve_manga_prompt_downstream_conflict(
            &panel,
            input.downstream_strategy,
            "优化提示词",
        )
        .map_err(AppError::InvalidArgument)?;
        self.optimize_manga_prompt_validated(
            MangaPromptOptimizationInput {
                panel,
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
        let panel = self
            .manga_page_prompt_by_id(prompt_id)
            .await?
            .ok_or_else(|| AppError::NotFound("整页漫画提示词不存在".to_string()))?;
        if panel.prompt_type != MANGA_PAGE_PROMPT_TYPE {
            return Err(AppError::InvalidArgument(
                "只能操作整页漫画提示词".to_string(),
            ));
        }
        Ok(panel)
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

use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPromptInput, MangaPagePromptSummary,
    MangaPipelinePreviewStep, MangaPromptDownstreamCleanupPort, OptimizeMangaPromptInput,
    UpdateMangaPromptInput,
};
use a4n_domain::manga::MangaPagePromptRecord;
use a4n_domain::{DownstreamConflictStrategy, project::required_text};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

use super::layout_rules::{
    bind_reference_asset_in_layout, manga_page_generation_layout,
    resolve_manga_prompt_downstream_conflict,
};
use super::{
    MangaGeneratePagePromptsInput, MangaPipelineRequestPreviewInput,
    MangaPipelineRequestPreviewStep, MangaPromptOptimizationInput, ensure_manga_source_can_write,
    manga_pipeline_fingerprint,
};
use crate::{
    OrchestratorContext, prompt_records::PromptRendererAdapter,
    shooting_script::ChapterShootingScriptSourceInput,
};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_manga_core::manga_page_prompt_summary;

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
        self.require_current_chapter_shooting_script(ChapterShootingScriptSourceInput {
            project_id,
            source_chapter_id: source.chapter_id,
            visual_plan_id: None,
        })
        .await?;
        Ok(())
    }

    pub async fn run_manga_pipeline_validated(
        &self,
        input: MangaPipelineApplicationInput<'_>,
    ) -> Result<Vec<MangaPagePromptRecord>, AppError> {
        let (source_result, character_portraits_result) = futures::join!(
            self.chapter_manga_source_by_number(input.project_id, input.chapter_number, true),
            self.manga_character_portrait_refs(input.project_id),
        );
        let source = source_result?;
        ensure_manga_source_can_write(&source, "启动漫画流水线")?;
        let character_portraits = character_portraits_result?;
        let workflow = self
            .begin_manga_pipeline_workflow_validated(
                input.project_id,
                source.chapter_id,
                &source.content,
                &character_portraits,
                input.task_id.to_string(),
                input.resume_from_checkpoint,
            )
            .await?;
        let result = self
            .generate_page_prompts(MangaGeneratePagePromptsInput {
                chapter_id: source.chapter_id,
                project_id: input.project_id,
                prompt_renderer: input.prompt_renderer,
                character_portraits,
                task_id: input.task_id,
                workflow_run_id: workflow.id,
                resume_from_checkpoint: input.resume_from_checkpoint,
            })
            .await;
        self.finalize_workflow_result(workflow.id, result, "漫画流水线")
            .await
    }

    pub async fn prepare_manga_pipeline_preview_validated(
        &self,
        input: MangaPipelinePreviewApplicationInput<'_>,
    ) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
        let (source_result, character_portraits_result) = futures::join!(
            self.chapter_manga_source_by_number(input.project_id, input.chapter_number, true),
            self.manga_character_portrait_refs(input.project_id),
        );
        let source = source_result?;
        ensure_manga_source_can_write(&source, "预览漫画流水线提示词")?;
        let character_portraits = character_portraits_result?;
        let workflow_fingerprint =
            manga_pipeline_fingerprint(&source.content, &character_portraits);
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
            character_portraits,
            workflow_run_id,
            resume_from_checkpoint: input.resume_from_checkpoint,
        })
        .await
    }

    pub async fn bind_manga_reference_image_validated(
        &self,
        user_id: i64,
        input: BindMangaReferenceImageInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptRecord, AppError> {
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
        let layout = manga_page_generation_layout(
            page_prompt.id,
            page_prompt.page_number,
            page_prompt.layout.as_ref(),
        )
        .map_err(AppError::InvalidArgument)?;
        let layout = bind_reference_asset_in_layout(layout, character_name, bound_asset)
            .map_err(AppError::InvalidArgument)?;
        let updated = self
            .update_manga_page_prompt_layout(page_prompt, serde_json::to_value(layout)?)
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
    ) -> Result<MangaPagePromptRecord, AppError> {
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

    pub async fn optimize_manga_prompt_record_validated(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptInput,
        prompt_renderer: &dyn PromptRenderer,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptRecord, AppError> {
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
    ) -> Result<MangaPagePromptRecord, AppError> {
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
        let (project_result, root_project_result) = futures::join!(
            self.verify_project_permission(family.project.id, user_id, required),
            self.verify_project_permission(family.root_project.id, user_id, required),
        );
        project_result?;
        root_project_result?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::manga::MangaPipelineExecutionApplicationPort for OrchestratorContext {
    async fn preflight(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, project_id)
            .await?;
        self.preflight_manga_pipeline_validated(project_id, chapter_number)
            .await
    }

    async fn run(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        task_id: String,
        resume_from_checkpoint: bool,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<(), AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        self.verify_project_or_derived_family_editor_validated(user_id, project_id)
            .await?;
        self.run_manga_pipeline_validated(MangaPipelineApplicationInput {
            project_id,
            chapter_number,
            task_id: &task_id,
            resume_from_checkpoint,
            prompt_renderer: &prompt_renderer,
        })
        .await
        .map(|_| ())
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::manga::MangaPipelinePreviewApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        resume_from_checkpoint: bool,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<Vec<MangaPipelinePreviewStep>, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        self.verify_project_or_derived_family_editor_validated(user_id, project_id)
            .await?;
        self.prepare_manga_pipeline_preview_validated(MangaPipelinePreviewApplicationInput {
            project_id,
            chapter_number,
            resume_from_checkpoint,
            prompt_renderer: &prompt_renderer,
        })
        .await?
        .into_iter()
        .map(manga_pipeline_preview_step)
        .collect()
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::manga::MangaRecordMutationApplicationPort for OrchestratorContext {
    async fn bind_reference_image(
        &self,
        user_id: i64,
        input: BindMangaReferenceImageInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptSummary, AppError> {
        let prompt = self
            .bind_manga_reference_image_validated(user_id, input, cleanup)
            .await?;
        manga_page_prompt_summary(&prompt).map_err(AppError::InvalidArgument)
    }

    async fn update_prompt(
        &self,
        user_id: i64,
        input: UpdateMangaPromptInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptSummary, AppError> {
        let prompt = self
            .update_manga_prompt_validated(user_id, input, cleanup)
            .await?;
        manga_page_prompt_summary(&prompt).map_err(AppError::InvalidArgument)
    }

    async fn delete_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        self.delete_manga_by_chapter_validated(user_id, project_id, chapter_id)
            .await
    }

    async fn delete_prompt(
        &self,
        user_id: i64,
        input: DeleteMangaPromptInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<(), AppError> {
        self.delete_manga_prompt_validated(
            user_id,
            input.prompt_id,
            input.downstream_strategy,
            cleanup,
        )
        .await
    }

    async fn optimize_prompt(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptInput,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptSummary, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        let prompt = self
            .optimize_manga_prompt_record_validated(user_id, input, &prompt_renderer, cleanup)
            .await?;
        manga_page_prompt_summary(&prompt).map_err(AppError::InvalidArgument)
    }
}

fn manga_pipeline_preview_step(
    step: MangaPipelineRequestPreviewStep,
) -> Result<MangaPipelinePreviewStep, AppError> {
    Ok(MangaPipelinePreviewStep {
        key: step.key,
        title: step.title,
        prompt_name: step.prompt_name,
        state: step.state,
        note: step.note,
        request: step.request.map(serde_json::to_value).transpose()?,
    })
}

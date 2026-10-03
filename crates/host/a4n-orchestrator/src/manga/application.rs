use crate::generation::events::GenerationEventPublisher;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPromptInput, MangaPagePromptSummary,
    MangaPipelinePreviewStep, MangaPromptDownstreamCleanupPort, OptimizeMangaPromptInput,
    UpdateMangaPromptInput,
};
use a4n_domain::{
    DownstreamConflictStrategy, manga::MangaPagePromptRecord, project::required_text,
};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::ProgressEvent;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::layout_rules::{
    bind_reference_asset_in_layout, bind_reference_asset_in_pipeline_layout,
    manga_page_generation_layout, manga_page_pipeline_layout,
    resolve_manga_prompt_downstream_conflict,
};
use super::{
    MangaGeneratePagePromptsInput, MangaPipelineRequestPreviewInput,
    MangaPipelineRequestPreviewStep, MangaPromptOptimizationInput, ensure_manga_source_can_write,
    manga_pipeline_fingerprint,
};
use crate::{OrchestratorContext, prompt_records::PromptRendererAdapter};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_manga_core::manga_page_prompt_summary;

pub struct MangaPipelineApplicationInput<'a> {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_number: i32,
    pub task_id: &'a str,
    pub resume_from_checkpoint: bool,
    pub stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub image_size: Option<a4n_domain::ImageSize>,
    pub image_generator: Option<&'a dyn a4n_api_contract::manga::MangaPipelinePageImageGenerator>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub progress_reporter: Option<&'a (dyn Fn(ProgressEvent) + Send + Sync)>,
    pub stream_reporter: Option<&'a (dyn Fn(&str, &str) + Send + Sync)>,
}

pub struct MangaPipelinePreviewApplicationInput<'a> {
    pub project_id: i64,
    pub chapter_number: i32,
    pub resume_from_checkpoint: bool,
    pub stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

enum MangaTaskEvent {
    Progress(ProgressEvent),
    Text { stream_id: String, delta: String },
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
        let (workflow, restore_from_checkpoint) = self
            .begin_manga_pipeline_workflow_validated(
                input.project_id,
                source.chapter_id,
                &source.content,
                &character_portraits,
                input.task_id.to_string(),
                input.resume_from_checkpoint,
                input.stop_after,
            )
            .await?;
        let result = self
            .generate_page_prompts(MangaGeneratePagePromptsInput {
                user_id: input.user_id,
                chapter_id: source.chapter_id,
                chapter_number: input.chapter_number,
                project_id: input.project_id,
                prompt_renderer: input.prompt_renderer,
                character_portraits,
                task_id: input.task_id,
                workflow_run_id: workflow.id,
                resume_from_checkpoint: restore_from_checkpoint,
                stop_after: input.stop_after,
                image_size: input.image_size,
                image_generator: input.image_generator,
                cancel_token: input.cancel_token,
                progress_reporter: input.progress_reporter,
                stream_reporter: input.stream_reporter,
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
            self.completed_manga_pipeline_stage_workflow(
                input.project_id,
                source.chapter_id,
                &workflow_fingerprint,
                input.stop_after,
            )
            .await?
            .map(|run| run.id)
        };
        self.preview_generate_page_prompt_requests(MangaPipelineRequestPreviewInput {
            chapter_id: source.chapter_id,
            project_id: input.project_id,
            prompt_renderer: input.prompt_renderer,
            character_portraits,
            workflow_run_id,
            resume_from_checkpoint: workflow_run_id.is_some(),
            stop_after: input.stop_after,
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
        let layout_value = page_prompt.layout.as_ref().ok_or_else(|| {
            AppError::InvalidArgument(format!("漫画页面提示词 #{} 缺少 layout", page_prompt.id))
        })?;
        let layout = match layout_value
            .get("schema_version")
            .and_then(serde_json::Value::as_str)
        {
            Some("manga_page_generation_v4") => {
                let layout =
                    manga_page_pipeline_layout(&page_prompt).map_err(AppError::InvalidArgument)?;
                serde_json::to_value(
                    bind_reference_asset_in_pipeline_layout(layout, character_name, bound_asset)
                        .map_err(AppError::InvalidArgument)?,
                )?
            }
            _ => {
                let layout = manga_page_generation_layout(&page_prompt)
                    .map_err(AppError::InvalidArgument)?;
                serde_json::to_value(
                    bind_reference_asset_in_layout(layout, character_name, bound_asset)
                        .map_err(AppError::InvalidArgument)?,
                )?
            }
        };
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
        self.verify_project_or_derived_family_access_for_role_validated(user_id, project_id, required)
        .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::manga::MangaPipelineExecutionApplicationPort for OrchestratorContext {
    async fn prepare_standalone_source(
        &self,
        user_id: i64,
        input: a4n_api_contract::manga::PrepareStandaloneComicSourceInput,
    ) -> Result<a4n_api_contract::manga::MangaStandaloneSourceSummary, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
        .await?;
        let chapter = a4n_db::project::prepare_standalone_comic_source_chapter(
            self.database.connection(),
            a4n_db::project::PrepareStandaloneComicSourceDbInput {
                project_id: input.project_id,
                asset_id: input.asset_id,
                version: input.version,
                start_section: input.start_section,
                end_section: input.end_section,
            },
        )
        .await
        .map_err(|error| match error {
            a4n_db::project::StandaloneComicSourceDbError::InvalidArgument(message) => {
                AppError::InvalidArgument(message)
            }
            a4n_db::project::StandaloneComicSourceDbError::InvalidState(message) => {
                AppError::InvalidState {
                    current: message,
                    operation: "准备独立漫画文本来源".into(),
                }
            }
            a4n_db::project::StandaloneComicSourceDbError::NotFound(message) => {
                AppError::NotFound(message)
            }
            a4n_db::project::StandaloneComicSourceDbError::Forbidden(message) => {
                AppError::Forbidden(message)
            }
            a4n_db::project::StandaloneComicSourceDbError::Database(error) => {
                AppError::Database(error)
            }
        })?;
        Ok(a4n_api_contract::manga::MangaStandaloneSourceSummary {
            chapter_id: chapter.id,
            chapter_number: chapter.chapter_number,
            title: chapter.title,
            content_sha256: a4n_domain::source_text_hash(&chapter.content),
            content: chapter.content,
            asset_id: input.asset_id,
            version: input.version,
            start_section: input.start_section,
            end_section: input.end_section,
        })
    }

    async fn start_task(
        &self,
        user_id: i64,
        input: &a4n_api_contract::manga::MangaPipelineStartInput,
    ) -> Result<
        a4n_api_contract::generation_tasks::GenerationTaskStart,
        a4n_api_contract::generation_tasks::GenerationTaskStartError,
    > {
        self.start_manga_pipeline_task(user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run_task(
        &self,
        user_id: i64,
        input: a4n_api_contract::manga::MangaPipelineStartInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
        page_image_generator: &dyn a4n_api_contract::manga::MangaPipelinePageImageGenerator,
    ) -> Result<(), String> {
        self.run_manga_pipeline_generation_task(
            user_id,
            input,
            handle,
            prompt_renderer,
            page_image_generator,
        )
        .await
    }

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
}

impl OrchestratorContext {
    pub async fn run_manga_pipeline_generation_task(
        &self,
        user_id: i64,
        input: a4n_api_contract::manga::MangaPipelineStartInput,
        handle: GenerationTaskHandle,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
        page_image_generator: &dyn a4n_api_contract::manga::MangaPipelinePageImageGenerator,
    ) -> Result<(), String> {
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let cancel_token = execution.permit.cancellation_token();
        let publisher = execution.publisher.clone();
        let (sender, mut receiver) = mpsc::unbounded_channel::<MangaTaskEvent>();
        let event_worker = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                match event {
                    MangaTaskEvent::Progress(event) => publisher
                        .progress(event)
                        .await
                        .map_err(|error| error.to_string())?,
                    MangaTaskEvent::Text { stream_id, delta } => publisher
                        .text_delta(&stream_id, &delta)
                        .await
                        .map_err(|error| error.to_string())?,
                }
            }
            Ok::<(), String>(())
        });
        let progress_sender = sender.clone();
        let progress_reporter = move |event: ProgressEvent| {
            let _ = progress_sender.send(MangaTaskEvent::Progress(event));
        };
        let stream_sender = sender.clone();
        let stream_reporter = move |stream_id: &str, delta: &str| {
            if !delta.is_empty() {
                let _ = stream_sender.send(MangaTaskEvent::Text {
                    stream_id: stream_id.to_string(),
                    delta: delta.to_string(),
                });
            }
        };
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        let result = self
            .run_manga_pipeline_validated(MangaPipelineApplicationInput {
                user_id,
                project_id: input.project_id,
                chapter_number: input.chapter_number,
                task_id: &handle.task_id,
                resume_from_checkpoint: input.resume_from_checkpoint,
                stop_after: input.stop_after,
                image_size: input.image_size,
                image_generator: Some(page_image_generator),
                cancel_token: Some(&cancel_token),
                prompt_renderer: &prompt_renderer,
                progress_reporter: Some(&progress_reporter),
                stream_reporter: Some(&stream_reporter),
            })
            .await;
        drop(progress_reporter);
        drop(stream_reporter);
        drop(sender);
        let event_result = event_worker
            .await
            .map_err(|error| format!("漫画 generation event publisher 异常退出: {error}"))?;
        if let Err(error) = event_result {
            return Err(execution
                .fail(crate::generation::generation_public_error(
                    "generation_event_invalid",
                    "manga_pipeline",
                    error,
                ))
                .await);
        }
        match result {
            Ok(rows) => {
                let source = self
                    .chapter_manga_source_by_number(input.project_id, input.chapter_number, false)
                    .await
                    .map_err(|error| error.to_string())?;
                let workflow_run = self
                    .latest_workflow_run_by_business_key(
                        &crate::manga::manga_pipeline_business_key(
                            input.project_id,
                            source.chapter_id,
                        ),
                    )
                    .await
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "漫画 workflow run 在任务完成时不存在".to_string())?;
                let result_ref = crate::generation::generation_result_ref(
                    "manga_pages",
                    workflow_run.id,
                    &rows
                        .iter()
                        .map(|row| (row.id, row.page_number, row.generated_image_id))
                        .collect::<Vec<_>>(),
                )
                .map_err(|error| error.to_string())?;
                execution
                    .complete_with_result(
                        "manga_pipeline",
                        result_ref,
                        format!("漫画页面流水线完成，共 {} 页", rows.len()),
                        Vec::new(),
                        None,
                    )
                    .await
            }
            Err(error) => Err(execution
                .fail(crate::generation::generation_public_error(
                    "provider_job_failed",
                    "manga_pipeline",
                    error.to_string(),
                ))
                .await),
        }
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
        stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<Vec<MangaPipelinePreviewStep>, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        self.verify_project_or_derived_family_editor_validated(user_id, project_id)
            .await?;
        self.prepare_manga_pipeline_preview_validated(MangaPipelinePreviewApplicationInput {
            project_id,
            chapter_number,
            resume_from_checkpoint,
            stop_after,
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

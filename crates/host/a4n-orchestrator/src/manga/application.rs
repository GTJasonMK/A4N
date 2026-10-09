use std::collections::HashMap;

use crate::generation::events::GenerationEventPublisher;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::manga::{
    BindMangaReferenceImageInput, DeleteMangaPageInput, DeleteMangaPipelineArtifactInput,
    DeleteMangaPromptInput, MangaPagePromptSummary, MangaPipelineArtifactStage,
    MangaPipelinePreviewStep, OptimizeMangaPromptInput, SaveMangaPipelineArtifactInput,
    UpdateMangaPromptInput,
};
use a4n_domain::{
    manga::{
        MangaPageDrawingPromptSchemaVersion, MangaPageDrawingPromptV2, MangaPagePromptRecord,
        MangaPageSlotsV1, MangaPipelineArtifactDraft,
    },
    project::required_text,
    source_text_hash,
};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::ProgressEvent;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use super::layout_rules::{
    manga_page_generation_layout, manga_page_pipeline_layout, mutate_reference_asset_in_layout,
    mutate_reference_asset_in_pipeline_layout,
};
use super::{
    MangaGeneratePagePromptsInput, MangaPipelineRequestPreviewInput,
    MangaPipelineRequestPreviewStep, MangaPromptOptimizationInput, ensure_manga_source_can_write,
    validate_manga_page_scope,
};
use crate::{OrchestratorContext, prompt_records::PromptRendererAdapter};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_manga_core::manga_page_prompt_summary;

pub struct MangaPipelineApplicationInput<'a> {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_number: i32,
    pub page_number: Option<u32>,
    pub task_id: &'a str,
    pub resume_from_checkpoint: bool,
    pub stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub run_stage: Option<MangaPipelineArtifactStage>,
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
    pub page_number: Option<u32>,
    pub resume_from_checkpoint: bool,
    pub stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
    pub run_stage: Option<MangaPipelineArtifactStage>,
    pub prompt_renderer: &'a dyn PromptRenderer,
}

enum MangaTaskEvent {
    Progress(ProgressEvent),
    Text { stream_id: String, delta: String },
}

async fn manga_page_prompt_summary_with_stale_stages(
    context: &OrchestratorContext,
    prompt: &MangaPagePromptRecord,
) -> Result<MangaPagePromptSummary, AppError> {
    let mut summary = manga_page_prompt_summary(prompt).map_err(AppError::InvalidArgument)?;
    let artifacts = context
        .manga_pipeline_artifacts_by_chapter(prompt.project_id, prompt.chapter_id)
        .await?;
    summary.stale_stages =
        a4n_manga_core::manga_page_prompt_stale_stages(prompt.id, prompt.page_number, &artifacts)
            .map_err(AppError::InvalidArgument)?;
    Ok(summary)
}

fn manga_pipeline_prompts_for_run(
    run_stage: Option<MangaPipelineArtifactStage>,
    stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
) -> Vec<&'static str> {
    use a4n_api_contract::manga::MangaPipelineStopAfter as StopAfter;
    if let Some(stage) = run_stage {
        return match stage {
            MangaPipelineArtifactStage::Preprocessing => vec!["manga_preprocessing"],
            MangaPipelineArtifactStage::PageSlots => vec!["manga_page_slots"],
            MangaPipelineArtifactStage::PageAdaptation => {
                vec!["manga_page_adaptation_plan"]
            }
            MangaPipelineArtifactStage::Storyboard => vec!["manga_page_storyboard"],
            MangaPipelineArtifactStage::ReferenceDiscovery => {
                vec!["manga_page_reference_discovery"]
            }
            MangaPipelineArtifactStage::DrawingPrompt => vec!["manga_page_drawing_prompt"],
            MangaPipelineArtifactStage::Image
            | MangaPipelineArtifactStage::PageContent
            | MangaPipelineArtifactStage::ReferenceBindings => Vec::new(),
        };
    }

    let mut prompt_names = vec!["manga_page_slots"];
    if stop_after == Some(StopAfter::PageSlots) {
        return prompt_names;
    }
    prompt_names.push("manga_page_adaptation_plan");
    if stop_after == Some(StopAfter::PageAdaptation) {
        return prompt_names;
    }
    prompt_names.push("manga_page_storyboard");
    if stop_after == Some(StopAfter::Storyboard) {
        return prompt_names;
    }
    prompt_names.push("manga_page_reference_discovery");
    if stop_after == Some(StopAfter::ReferenceDiscovery) {
        return prompt_names;
    }
    prompt_names.push("manga_page_drawing_prompt");
    prompt_names
}

fn validate_manga_run_stage_scope(
    run_stage: Option<MangaPipelineArtifactStage>,
    page_number: Option<u32>,
) -> Result<(), AppError> {
    let Some(stage) = run_stage else {
        return Ok(());
    };
    if stage.page_scoped() != page_number.is_some() {
        return Err(AppError::InvalidArgument(if stage.page_scoped() {
            "页级漫画步骤必须指定页码".into()
        } else {
            "章节级漫画步骤不能指定页码".into()
        }));
    }
    match stage {
        MangaPipelineArtifactStage::PageContent => Err(AppError::InvalidArgument(
            "页面内容通过分页产物编辑，不单独生成".into(),
        )),
        MangaPipelineArtifactStage::ReferenceBindings => Err(AppError::InvalidArgument(
            "参考图绑定通过参考图面板管理".into(),
        )),
        _ => Ok(()),
    }
}

fn validate_manga_storyboard_prompt_contract(prompt: &str) -> Result<(), AppError> {
    if prompt.contains("\"layer_index\"") && !prompt.contains("\"layer_number\"") {
        return Ok(());
    }
    Err(AppError::InvalidArgument(
        "当前 manga_page_storyboard 提示词未使用合同字段 `layer_index`；已在任务创建前阻止生成，请修正或重置提示词后重试。".into(),
    ))
}

impl OrchestratorContext {
    pub async fn preflight_manga_pipeline_validated(
        &self,
        project_id: i64,
        chapter_number: i32,
        page_number: Option<u32>,
        stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
        run_stage: Option<MangaPipelineArtifactStage>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<(), AppError> {
        validate_manga_page_scope(page_number, stop_after)?;
        validate_manga_run_stage_scope(run_stage, page_number)?;
        let prompt_names = manga_pipeline_prompts_for_run(run_stage, stop_after);
        let source = self
            .chapter_manga_source_by_number(project_id, chapter_number, true)
            .await?;
        ensure_manga_source_can_write(&source, "启动漫画流水线")?;
        if run_stage == Some(MangaPipelineArtifactStage::Preprocessing) {
            for prompt_name in prompt_names {
                prompt_renderer
                    .render_prompt(prompt_name, &HashMap::new(), Some(project_id))
                    .await?;
            }
            return Ok(());
        }
        let preprocessing = self
            .manga_pipeline_artifact(
                project_id,
                source.chapter_id,
                MangaPipelineArtifactStage::Preprocessing,
                None,
            )
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_preprocessing_missing".into(),
                operation: "请先完成漫画预处理".into(),
            })?;
        let preprocessing_input_fingerprint = self
            .current_manga_preprocessing_input_fingerprint(
                project_id,
                &a4n_domain::source_text_hash(&source.content),
                source.chapter_number,
            )
            .await?;
        if preprocessing.stale || preprocessing.input_fingerprint != preprocessing_input_fingerprint
        {
            return Err(AppError::InvalidState {
                current: "manga_preprocessing_stale".into(),
                operation: "原文章节已变化，请重新运行漫画预处理".into(),
            });
        }
        let adaptation_source = a4n_db::project::resolve_chapter_source(
            self.database.executor(),
            project_id,
            source.chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        self.require_adopted_manga_adaptation(
            &adaptation_source,
            &preprocessing.output_fingerprint,
        )
        .await?;
        let requested_page_number = page_number
            .map(|page_number| {
                i32::try_from(page_number)
                    .map_err(|_| AppError::InvalidArgument("漫画页码超出 i32 范围".into()))
            })
            .transpose()?;
        if let Some(page_number) = requested_page_number {
            let pages = self
                .manga_page_prompts_by_chapter(project_id, source.chapter_id)
                .await?;
            if !pages.iter().any(|page| page.page_number == page_number) {
                return Err(AppError::InvalidArgument(format!(
                    "漫画第 {page_number} 页不存在，无法启动单页流水线"
                )));
            }
        }
        for prompt_name in prompt_names {
            let prompt = prompt_renderer
                .render_prompt(prompt_name, &HashMap::new(), Some(project_id))
                .await?;
            if prompt_name == "manga_page_storyboard" {
                validate_manga_storyboard_prompt_contract(&prompt)?;
            }
        }
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
        let asset_project_ids = self
            .manga_reference_asset_project_ids(input.project_id)
            .await?;
        let available_reference_images = self
            .video_reference_candidate_images(&asset_project_ids, source.chapter_id)
            .await?;
        let (workflow, restore_from_checkpoint) = self
            .begin_manga_pipeline_workflow_validated(
                input.project_id,
                source.chapter_id,
                &source.content,
                &character_portraits,
                &available_reference_images,
                input.task_id.to_string(),
                input.resume_from_checkpoint,
                input.stop_after,
                input.run_stage,
                input.page_number,
            )
            .await?;
        let result = self
            .generate_page_prompts(MangaGeneratePagePromptsInput {
                user_id: input.user_id,
                chapter_id: source.chapter_id,
                chapter_number: input.chapter_number,
                page_number: input.page_number,
                project_id: input.project_id,
                prompt_renderer: input.prompt_renderer,
                character_portraits,
                available_reference_images,
                task_id: input.task_id,
                workflow_run_id: workflow.id,
                resume_from_checkpoint: restore_from_checkpoint,
                stop_after: input.stop_after,
                run_stage: input.run_stage,
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
        validate_manga_page_scope(input.page_number, input.stop_after)?;
        validate_manga_run_stage_scope(input.run_stage, input.page_number)?;
        let (source_result, character_portraits_result) = futures::join!(
            self.chapter_manga_source_by_number(input.project_id, input.chapter_number, true),
            self.manga_character_portrait_refs(input.project_id),
        );
        let source = source_result?;
        ensure_manga_source_can_write(&source, "预览漫画流水线提示词")?;
        let character_portraits = character_portraits_result?;
        let asset_project_ids = self
            .manga_reference_asset_project_ids(input.project_id)
            .await?;
        let available_reference_images = self
            .video_reference_candidate_images(&asset_project_ids, source.chapter_id)
            .await?;
        let workflow_fingerprint = crate::manga::manga_pipeline_run_fingerprint(
            &source.content,
            &character_portraits,
            &available_reference_images,
            input.run_stage,
        );
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
                input.page_number,
            )
            .await?
            .map(|run| run.id)
        };
        self.preview_generate_page_prompt_requests(MangaPipelineRequestPreviewInput {
            chapter_id: source.chapter_id,
            project_id: input.project_id,
            page_number: input.page_number,
            prompt_renderer: input.prompt_renderer,
            character_portraits,
            available_reference_images,
            workflow_run_id,
            resume_from_checkpoint: workflow_run_id.is_some(),
            stop_after: input.stop_after,
            run_stage: input.run_stage,
        })
        .await
    }

    pub async fn bind_manga_reference_image_validated(
        &self,
        user_id: i64,
        input: BindMangaReferenceImageInput,
    ) -> Result<MangaPagePromptRecord, AppError> {
        let character_name =
            required_text(&input.character_name, "角色名称").map_err(AppError::InvalidArgument)?;
        let page_prompt = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, page_prompt.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(page_prompt.chapter_id, page_prompt.project_id, false)
            .await?;
        let operation = match input.action {
            a4n_domain::manga::MangaReferenceBindingAction::AddUserImage => "添加漫画用户参考图",
            a4n_domain::manga::MangaReferenceBindingAction::RemoveUserImage => "移除漫画用户参考图",
        };
        ensure_manga_source_can_write(&source, operation)?;
        let asset = match input.action {
            a4n_domain::manga::MangaReferenceBindingAction::RemoveUserImage => None,
            _ => Some(
                self.manga_reference_bound_asset(
                    page_prompt.project_id,
                    input.asset_kind,
                    input.asset_id,
                )
                .await?,
            ),
        };
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
                    mutate_reference_asset_in_pipeline_layout(
                        layout,
                        character_name,
                        input.asset_kind,
                        input.asset_id,
                        input.action,
                        asset,
                    )
                    .map_err(AppError::InvalidArgument)?,
                )?
            }
            _ => {
                let layout = manga_page_generation_layout(&page_prompt)
                    .map_err(AppError::InvalidArgument)?;
                serde_json::to_value(
                    mutate_reference_asset_in_layout(
                        layout,
                        character_name,
                        input.asset_kind,
                        input.asset_id,
                        input.action,
                        asset,
                    )
                    .map_err(AppError::InvalidArgument)?,
                )?
            }
        };
        let updated = self
            .update_manga_page_prompt_layout(page_prompt, layout)
            .await?;
        let pipeline_layout =
            manga_page_pipeline_layout(&updated).map_err(AppError::InvalidArgument)?;
        let existing = self
            .manga_pipeline_artifact(
                updated.project_id,
                updated.chapter_id,
                MangaPipelineArtifactStage::ReferenceBindings,
                Some(
                    u32::try_from(updated.page_number)
                        .map_err(|_| AppError::InvalidArgument("漫画页码无效".into()))?,
                ),
            )
            .await?;
        self.upsert_manga_pipeline_artifact(MangaPipelineArtifactDraft {
            project_id: updated.project_id,
            chapter_id: updated.chapter_id,
            stage: MangaPipelineArtifactStage::ReferenceBindings,
            page_number: Some(
                u32::try_from(updated.page_number)
                    .map_err(|_| AppError::InvalidArgument("漫画页码无效".into()))?,
            ),
            input_fingerprint: existing.as_ref().map_or_else(
                || {
                    source_text_hash(&format!(
                        "manual:reference_bindings:{}:{}",
                        source_text_hash(&source.content),
                        updated.page_number
                    ))
                },
                |artifact| artifact.input_fingerprint.clone(),
            ),
            payload: serde_json::to_value(pipeline_layout.reference_bindings)?,
            stale: existing.as_ref().is_some_and(|artifact| artifact.stale),
        })
        .await?;
        Ok(updated)
    }

    pub async fn update_manga_prompt_validated(
        &self,
        user_id: i64,
        input: UpdateMangaPromptInput,
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
        let prompt_id = page_prompt.id;
        self.save_manga_pipeline_artifact_validated(
            user_id,
            drawing_prompt_artifact_input(&page_prompt, image_prompt.to_string())?,
        )
        .await?;
        self.required_manga_page_prompt(prompt_id).await
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
    ) -> Result<(), AppError> {
        let page_prompt = self.required_manga_page_prompt(prompt_id).await?;
        self.verify_manga_permission(user_id, page_prompt.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(page_prompt.chapter_id, page_prompt.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "删除漫画绘图提示词")?;
        let page_number = u32::try_from(page_prompt.page_number)
            .map_err(|_| AppError::InvalidArgument("漫画页码无效".into()))?;
        let expected_artifact_ids = self
            .manga_pipeline_artifact_deletion_plan(
                page_prompt.project_id,
                page_prompt.chapter_id,
                MangaPipelineArtifactStage::DrawingPrompt,
                Some(page_number),
            )
            .await?
            .into_iter()
            .map(|artifact| artifact.id)
            .collect();
        self.delete_manga_pipeline_artifact_validated(
            user_id,
            DeleteMangaPipelineArtifactInput {
                project_id: page_prompt.project_id,
                chapter_id: page_prompt.chapter_id,
                stage: MangaPipelineArtifactStage::DrawingPrompt,
                page_number: Some(page_number),
                expected_artifact_ids,
            },
        )
        .await
    }

    pub async fn delete_manga_page_validated(
        &self,
        user_id: i64,
        input: DeleteMangaPageInput,
    ) -> Result<(), AppError> {
        self.verify_manga_permission(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        let source = self
            .chapter_manga_source_by_id(input.chapter_id, input.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "删除已移出分页计划的漫画页面")?;
        let slots = self
            .manga_pipeline_artifact(
                input.project_id,
                input.chapter_id,
                MangaPipelineArtifactStage::PageSlots,
                None,
            )
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_page_slots_missing".into(),
                operation: "只能删除已被新分页计划移出的页面".into(),
            })?;
        if slots.stale {
            return Err(AppError::InvalidState {
                current: "manga_page_slots_stale".into(),
                operation: "分页计划已过期，请先保存新的分页计划".into(),
            });
        }
        let slots: MangaPageSlotsV1 = serde_json::from_value(slots.payload)?;
        if slots
            .pages
            .iter()
            .any(|page| page.page_number == input.page_number)
        {
            return Err(AppError::InvalidArgument(
                "当前分页计划仍包含该页面，不能删除".into(),
            ));
        }
        self.delete_manga_page_by_number(
            input.project_id,
            input.chapter_id,
            i32::try_from(input.page_number)
                .map_err(|_| AppError::InvalidArgument("漫画页码超出范围".into()))?,
        )
        .await
    }

    pub async fn optimize_manga_prompt_record_validated(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<MangaPagePromptRecord, AppError> {
        let page_prompt = self.required_manga_page_prompt(input.prompt_id).await?;
        self.verify_manga_permission(user_id, page_prompt.project_id, ProjectAccessRole::Editor)
            .await?;
        let prompt_id = page_prompt.id;
        let optimized = self
            .optimize_manga_prompt_validated(
                MangaPromptOptimizationInput {
                    page_prompt,
                    instruction: input.instruction,
                },
                prompt_renderer,
            )
            .await?;
        self.save_manga_pipeline_artifact_validated(
            user_id,
            drawing_prompt_artifact_input(&optimized, optimized.image_prompt.clone())?,
        )
        .await?;
        self.required_manga_page_prompt(prompt_id).await
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
        self.verify_project_or_derived_family_access_for_role_validated(
            user_id, project_id, required,
        )
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
            self.database.executor(),
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
                AppError::database(error)
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
        page_number: Option<u32>,
        stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
        run_stage: Option<MangaPipelineArtifactStage>,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<(), AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, project_id)
            .await?;
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        self.preflight_manga_pipeline_validated(
            project_id,
            chapter_number,
            page_number,
            stop_after,
            run_stage,
            &prompt_renderer,
        )
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
                page_number: input.page_number,
                task_id: &handle.task_id,
                resume_from_checkpoint: input.resume_from_checkpoint,
                stop_after: input.stop_after,
                run_stage: input.run_stage,
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
                        input.page_number.map_or_else(
                            || format!("漫画页面流水线完成，共 {} 页", rows.len()),
                            |page_number| format!("漫画第 {page_number} 页流水线已完成"),
                        ),
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
        page_number: Option<u32>,
        resume_from_checkpoint: bool,
        stop_after: Option<a4n_api_contract::manga::MangaPipelineStopAfter>,
        run_stage: Option<MangaPipelineArtifactStage>,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<Vec<MangaPipelinePreviewStep>, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        self.verify_project_or_derived_family_editor_validated(user_id, project_id)
            .await?;
        self.prepare_manga_pipeline_preview_validated(MangaPipelinePreviewApplicationInput {
            project_id,
            chapter_number,
            page_number,
            resume_from_checkpoint,
            stop_after,
            run_stage,
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
    async fn delete_page(&self, user_id: i64, input: DeleteMangaPageInput) -> Result<(), AppError> {
        self.delete_manga_page_validated(user_id, input).await
    }

    async fn bind_reference_image(
        &self,
        user_id: i64,
        input: BindMangaReferenceImageInput,
    ) -> Result<MangaPagePromptSummary, AppError> {
        let prompt = self
            .bind_manga_reference_image_validated(user_id, input)
            .await?;
        manga_page_prompt_summary_with_stale_stages(self, &prompt).await
    }

    async fn update_prompt(
        &self,
        user_id: i64,
        input: UpdateMangaPromptInput,
    ) -> Result<MangaPagePromptSummary, AppError> {
        let prompt = self.update_manga_prompt_validated(user_id, input).await?;
        manga_page_prompt_summary_with_stale_stages(self, &prompt).await
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
    ) -> Result<(), AppError> {
        self.delete_manga_prompt_validated(user_id, input.prompt_id)
            .await
    }

    async fn optimize_prompt(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptInput,
        prompt_renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<MangaPagePromptSummary, AppError> {
        let prompt_renderer = PromptRendererAdapter(prompt_renderer);
        let prompt = self
            .optimize_manga_prompt_record_validated(user_id, input, &prompt_renderer)
            .await?;
        manga_page_prompt_summary_with_stale_stages(self, &prompt).await
    }
}

fn drawing_prompt_artifact_input(
    page_prompt: &MangaPagePromptRecord,
    image_prompt: String,
) -> Result<SaveMangaPipelineArtifactInput, AppError> {
    let page_number = u32::try_from(page_prompt.page_number)
        .map_err(|_| AppError::InvalidArgument("漫画页码无效".into()))?;
    Ok(SaveMangaPipelineArtifactInput {
        project_id: page_prompt.project_id,
        chapter_id: page_prompt.chapter_id,
        stage: MangaPipelineArtifactStage::DrawingPrompt,
        page_number: Some(page_number),
        payload: serde_json::to_value(MangaPageDrawingPromptV2 {
            schema_version: MangaPageDrawingPromptSchemaVersion::V2,
            page_number,
            image_prompt,
        })?,
    })
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

#[cfg(test)]
mod preflight_tests {
    use super::{
        manga_pipeline_prompts_for_run, validate_manga_run_stage_scope,
        validate_manga_storyboard_prompt_contract,
    };
    use a4n_api_contract::manga::MangaPipelineStopAfter;
    use a4n_domain::manga::MangaPipelineArtifactStage;

    #[test]
    fn storyboard_preflight_requires_the_contract_field() {
        assert!(validate_manga_storyboard_prompt_contract(r#"{"layer_index": 1}"#).is_ok());
        assert!(validate_manga_storyboard_prompt_contract(r#"{"layer_number": 1}"#).is_err());
        assert!(
            validate_manga_storyboard_prompt_contract(r#"{"layer_index": 1, "layer_number": 1}"#)
                .is_err()
        );
    }

    #[test]
    fn preflight_only_renders_prompts_used_by_the_requested_stages() {
        assert_eq!(
            manga_pipeline_prompts_for_run(
                Some(MangaPipelineArtifactStage::Image),
                Some(MangaPipelineStopAfter::Image),
            ),
            Vec::<&str>::new(),
        );
        assert_eq!(
            manga_pipeline_prompts_for_run(
                Some(MangaPipelineArtifactStage::ReferenceDiscovery),
                Some(MangaPipelineStopAfter::ReferenceDiscovery),
            ),
            ["manga_page_reference_discovery"],
        );
        assert_eq!(
            manga_pipeline_prompts_for_run(None, Some(MangaPipelineStopAfter::PageAdaptation),),
            ["manga_page_slots", "manga_page_adaptation_plan"],
        );
        assert_eq!(
            manga_pipeline_prompts_for_run(None, Some(MangaPipelineStopAfter::Storyboard),),
            [
                "manga_page_slots",
                "manga_page_adaptation_plan",
                "manga_page_storyboard",
            ],
        );
    }

    #[test]
    fn manual_artifact_stages_cannot_be_started_as_generation_runs() {
        assert!(
            validate_manga_run_stage_scope(Some(MangaPipelineArtifactStage::PageContent), Some(1),)
                .is_err()
        );
        assert!(
            validate_manga_run_stage_scope(
                Some(MangaPipelineArtifactStage::ReferenceBindings),
                Some(1),
            )
            .is_err()
        );
        assert!(
            validate_manga_run_stage_scope(Some(MangaPipelineArtifactStage::Image), Some(1),)
                .is_ok()
        );
    }
}

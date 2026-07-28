use a4n_core::{
    chapter_analysis::validate_saved_chapter_analysis,
    error::AppError,
    models::{chapter::chapter, generation::workflow_run},
};
use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    chapter_content::ChapterDerivedCleanupHostPort,
    character::{
        CharacterDiscoveryInput, CharacterDiscoveryProgress, CharacterProfileBatchProgress,
        CharacterProfileSyncStageTracePort,
    },
    object_discovery::{ObjectDiscoveryInput, ObjectDiscoveryProgress},
    scene_profile::{SceneProfileSyncHostPort, SceneProfileSyncProgress},
    workflow::{
        CompletedWorkflowCheckpointInput, WorkflowRunFinalizeOutcome, required_checkpoint_field,
        required_checkpoint_json,
    },
};

use super::{
    ChapterAnalysisInput, chapter_analysis_phase_fingerprint, chapter_analysis_resume_block_reason,
    chapter_analysis_run_input_matches, chapter_analysis_workflow_business_key,
    chapter_analysis_workflow_run_input, ensure_chapter_analysis_character_sync_completed,
    ensure_previous_chapter_analysis_complete_from_rows, require_completed_chapter_analysis_phase,
};

pub const CHAPTER_ANALYSIS_APPLICATION_PROGRESS_TOTAL: u32 = 100;
const STEP_FACTS: &str = "chapter_analysis.facts";
const STEP_DERIVED_SYNC: &str = "chapter_analysis.derived_sync";
const STEP_SYNC_OUTPUTS: &str = "chapter_analysis.sync_outputs";
const STEP_OBJECT_DISCOVERY: &str = "chapter_analysis.object_discovery";
const STEP_CHARACTER_DISCOVERY: &str = "chapter_analysis.character_discovery";
const STEP_SCENE_SYNC: &str = "chapter_analysis.scene_sync";
const STEP_CHARACTER_SYNC: &str = "chapter_analysis.character_sync";
const STEP_REFRESH_STATUS: &str = "chapter_analysis.refresh_status";

pub type ChapterAnalysisApplicationProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy, Default)]
pub struct ChapterAnalysisApplicationProgress<'a> {
    pub reporter: Option<ChapterAnalysisApplicationProgressReporter<'a>>,
    pub cancel_token: Option<&'a CancellationToken>,
}

#[derive(Debug, Clone)]
pub struct ChapterAnalysisPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: Option<String>,
    pub status: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

impl ChapterAnalysisApplicationProgress<'_> {
    fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(
                current,
                CHAPTER_ANALYSIS_APPLICATION_PROGRESS_TOTAL,
                message,
            )?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }

    fn subprogress(
        &self,
        current: u32,
        total: u32,
        span_start: u32,
        span_end: u32,
        message: &str,
    ) -> Result<(), AppError> {
        let total = total.max(1);
        let span_width = span_end
            .checked_sub(span_start)
            .ok_or_else(|| AppError::InvalidArgument("章节分析进度区间顺序无效".to_string()))?;
        let offset = u64::from(span_width) * u64::from(current.min(total)) / u64::from(total);
        let mapped = span_start
            .checked_add(u32::try_from(offset).map_err(|_| {
                AppError::InvalidArgument("章节分析子进度超出 u32 范围".to_string())
            })?)
            .ok_or_else(|| AppError::InvalidArgument("章节分析进度溢出".to_string()))?;
        if let Some(reporter) = self.reporter {
            reporter(mapped, CHAPTER_ANALYSIS_APPLICATION_PROGRESS_TOTAL, message)?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }
}

#[async_trait]
pub trait ChapterAnalysisHostPort: ChapterDerivedCleanupHostPort + Send + Sync {
    async fn sync_chapter_rag_records(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
    ) -> Result<(), AppError>;

    fn scene_profile_sync_host_port(&self) -> &dyn SceneProfileSyncHostPort;
}

impl OrchestratorContext {
    pub async fn prepare_chapter_analysis_preview_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<(chapter::Model, Vec<ChapterAnalysisPreviewStep>), AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        let mut steps = self
            .build_chapter_analysis_requests(chapter_id, prompt_renderer)
            .await?
            .into_iter()
            .map(|step| ChapterAnalysisPreviewStep {
                key: step.key,
                title: step.title,
                prompt_name: Some(step.prompt_name),
                status: "ready".to_string(),
                note: None,
                request: Some(step.request),
            })
            .collect::<Vec<_>>();
        steps.push(blocked_preview_step(
            "object_discovery",
            "分析后物体档案发现请求",
            Some("object_discovery"),
            "该请求依赖结构化分析保存后的章节数据，发送前无法提前构造真实请求",
        ));
        let character_preview = self
            .prepare_character_discovery_request_preview_validated(
                CharacterDiscoveryInput {
                    project_id: chapter.project_id,
                    chapter_number: Some(chapter.chapter_number),
                },
                prompt_renderer,
            )
            .await?;
        steps.push(ChapterAnalysisPreviewStep {
            key: "character_discovery".to_string(),
            title: "角色发现初始 LLM 请求".to_string(),
            prompt_name: Some("character_discovery".to_string()),
            status: "ready".to_string(),
            note: Some(format!(
                "结构化分析保存后会继续执行该请求；分析章节数：{}",
                character_preview.analyzed_chapter_count
            )),
            request: Some(character_preview.request),
        });
        steps.extend(blocked_scene_preview_steps(
            chapter.chapter_number,
            "该请求依赖结构化分析保存后的章节数据，发送前无法提前构造真实请求",
        ));
        steps.push(blocked_preview_step(
            "analysis_character_sync",
            "分析后角色档案同步请求",
            Some("character_sync"),
            "该请求依赖结构化分析输出和角色发现模型返回结果，发送前无法提前构造真实请求",
        ));
        Ok((chapter, steps))
    }

    pub async fn prepare_resume_chapter_analysis_preview_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<(chapter::Model, Vec<ChapterAnalysisPreviewStep>), AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, true)
            .await?;
        let object_preview = self
            .prepare_object_discovery_request_preview_validated(
                ObjectDiscoveryInput {
                    project_id: chapter.project_id,
                    chapter_number: Some(chapter.chapter_number),
                },
                prompt_renderer,
            )
            .await?;
        let mut steps = vec![ChapterAnalysisPreviewStep {
            key: "object_discovery".to_string(),
            title: "物体档案发现初始 LLM 请求".to_string(),
            prompt_name: Some("object_discovery".to_string()),
            status: "ready".to_string(),
            note: Some(format!(
                "继续同步会先重新执行物体档案发现；分析章节数：{}",
                object_preview.analyzed_chapter_count
            )),
            request: Some(object_preview.request),
        }];
        let character_preview = self
            .prepare_character_discovery_request_preview_validated(
                CharacterDiscoveryInput {
                    project_id: chapter.project_id,
                    chapter_number: Some(chapter.chapter_number),
                },
                prompt_renderer,
            )
            .await?;
        steps.push(ChapterAnalysisPreviewStep {
            key: "character_discovery".to_string(),
            title: "角色发现初始 LLM 请求".to_string(),
            prompt_name: Some("character_discovery".to_string()),
            status: "ready".to_string(),
            note: Some(format!(
                "继续同步会先重新执行角色发现；分析章节数：{}",
                character_preview.analyzed_chapter_count
            )),
            request: Some(character_preview.request),
        });
        steps.extend(
            self.prepare_scene_profile_sync_preview_validated(
                chapter.project_id,
                Some(chapter.chapter_number),
                prompt_renderer,
                "ready",
                Some(
                    "继续同步会先执行场景档案候选识别；后续场景同步请求依赖该阶段返回结果"
                        .to_string(),
                ),
            )
            .await?
            .into_iter()
            .map(|step| ChapterAnalysisPreviewStep {
                key: step.key,
                title: step.title,
                prompt_name: step.prompt_name,
                status: step.state,
                note: step.note,
                request: step.request,
            }),
        );
        steps.push(blocked_preview_step(
            "analysis_character_sync",
            "分析后角色档案同步请求",
            Some("character_sync"),
            "该请求依赖角色发现模型返回结果；当前只能确认会在发现结果返回后构造",
        ));
        Ok((chapter, steps))
    }

    pub async fn preflight_chapter_analysis_validated(
        &self,
        chapter_id: i64,
        require_saved_analysis: bool,
    ) -> Result<chapter::Model, AppError> {
        let chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        ensure_chapter_not_generating(&chapter)?;
        if !chapter.has_available_content() {
            return Err(AppError::InvalidState {
                current: "empty_chapter_content".to_string(),
                operation: if require_saved_analysis {
                    "继续同步章节结构化分析".to_string()
                } else {
                    "生成章节结构化分析".to_string()
                },
            });
        }
        if require_saved_analysis {
            let analysis =
                chapter
                    .analysis_data
                    .as_ref()
                    .ok_or_else(|| AppError::InvalidState {
                        current: "missing_chapter_analysis".to_string(),
                        operation: "继续同步章节结构化分析".to_string(),
                    })?;
            validate_saved_chapter_analysis(analysis).map_err(|error| AppError::InvalidState {
                current: format!("invalid_saved_chapter_analysis: {error}"),
                operation: "继续同步章节结构化分析".to_string(),
            })?;
            self.ensure_chapter_analysis_sync_resume_needed(&chapter)
                .await?;
        }
        self.ensure_previous_chapter_analysis_complete(&chapter, "章节结构化分析")
            .await?;
        Ok(chapter)
    }

    pub async fn run_chapter_analysis_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        self.run_chapter_analysis_inner(
            chapter_id,
            true,
            prompt_renderer,
            host_port,
            progress,
            character_trace_port,
        )
        .await
    }

    pub async fn run_chapter_analysis_after_content_postprocess_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        self.run_chapter_analysis_inner(
            chapter_id,
            false,
            prompt_renderer,
            host_port,
            progress,
            character_trace_port,
        )
        .await
    }

    pub async fn resume_chapter_analysis_sync_validated(
        &self,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        progress.report(0, "正在检查已保存的章节分析")?;
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, true)
            .await?;
        let workflow_run = self
            .begin_workflow_run_validated(chapter_analysis_workflow_run_input(
                &chapter,
                STEP_DERIVED_SYNC,
            ))
            .await?;
        progress.report(45, "已找到结构化分析，正在继续同步派生数据")?;
        let result = self
            .sync_project_state_after_chapter_analysis(
                &chapter,
                workflow_run.id,
                prompt_renderer,
                host_port,
                progress,
                character_trace_port,
            )
            .await;
        self.finalize_chapter_analysis_application(workflow_run.id, result)
            .await?;
        self.chapter_by_id_for_analysis(chapter_id).await
    }

    pub async fn resume_chapter_analysis_workflow_run_validated(
        &self,
        run: &workflow_run::Model,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError> {
        if run.workflow_kind != "chapter_analysis" {
            return Err(AppError::InvalidArgument(format!(
                "工作流类型 {} 不是章节结构化分析",
                run.workflow_kind
            )));
        }
        if !run.can_resume {
            return Err(AppError::InvalidState {
                current: run.status.clone(),
                operation: run
                    .resume_block_reason
                    .clone()
                    .unwrap_or_else(|| "恢复章节结构化分析".to_string()),
            });
        }
        let chapter_id = run.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument("章节结构化分析工作流缺少 chapter_id".to_string())
        })?;
        let chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        ensure_chapter_not_generating(&chapter)?;
        if !chapter_analysis_run_input_matches(&run.input_fingerprint_json, &chapter) {
            return Err(AppError::InvalidState {
                current: "chapter_analysis_input_changed".to_string(),
                operation: "从旧的章节结构化分析断点恢复".to_string(),
            });
        }

        if chapter.analysis_data.is_some() {
            self.resume_chapter_analysis_sync_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                ChapterAnalysisApplicationProgress::default(),
                character_trace_port,
            )
            .await?;
        } else {
            self.run_chapter_analysis_inner(
                chapter_id,
                false,
                prompt_renderer,
                host_port,
                ChapterAnalysisApplicationProgress::default(),
                character_trace_port,
            )
            .await?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_chapter_analysis_inner(
        &self,
        chapter_id: i64,
        cleanup_before_analysis: bool,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        progress.report(0, "正在检查章节正文")?;
        let mut chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.preflight_analysis_character_profile_sync_validated(
            chapter.project_id,
            chapter.chapter_number,
        )
        .await?;

        if cleanup_before_analysis {
            progress.report(5, "正在清理旧的章节派生数据")?;
            self.postprocess_chapter_before_analysis(&chapter, host_port)
                .await?;
            chapter = self.chapter_by_id_for_analysis(chapter_id).await?;
        }

        progress.report(10, "正在读取章节并校验角色同步前置条件")?;
        let workflow_run = self
            .begin_workflow_run_validated(chapter_analysis_workflow_run_input(&chapter, STEP_FACTS))
            .await?;
        progress.report(20, "正在调用模型生成结构化分析")?;
        let analysis_reporter = |current: u32, total: u32, message: &str| {
            progress.subprogress(current, total, 20, 45, message)
        };
        let mut mapped_analysis_progress =
            |current: u32, total: u32, message: &str| analysis_reporter(current, total, message);
        let result = async {
            self.analyze_and_store_chapter(ChapterAnalysisInput {
                chapter_id,
                prompt_renderer,
                progress: Some(&mut mapped_analysis_progress),
                cancel_token: progress.cancel_token,
                workflow_run_id: Some(workflow_run.id),
            })
            .await?;
            progress.report(45, "结构化分析已保存，正在读取章节")?;
            let updated = self.chapter_by_id_for_analysis(chapter_id).await?;
            self.sync_project_state_after_chapter_analysis(
                &updated,
                workflow_run.id,
                prompt_renderer,
                host_port,
                progress,
                character_trace_port,
            )
            .await?;
            Ok(updated)
        }
        .await;
        self.finalize_chapter_analysis_application(workflow_run.id, result)
            .await
    }

    async fn postprocess_chapter_before_analysis(
        &self,
        chapter: &chapter::Model,
        host_port: &dyn ChapterAnalysisHostPort,
    ) -> Result<(), AppError> {
        self.postprocess_chapter_after_content_change_validated(host_port, chapter.id)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn sync_project_state_after_chapter_analysis(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError> {
        self.mark_workflow_current_step(workflow_run_id, STEP_DERIVED_SYNC)
            .await?;
        progress.report(50, "正在同步章节分析、伏笔和 RAG")?;
        self.sync_analysis_outputs_phase(chapter, workflow_run_id, host_port, progress)
            .await?;
        self.sync_object_discovery_phase(chapter, workflow_run_id, prompt_renderer, progress)
            .await?;
        let discovery_result = self
            .sync_character_discovery_phase(chapter, workflow_run_id, prompt_renderer, progress)
            .await?;
        self.sync_scene_profiles_phase(
            chapter,
            workflow_run_id,
            prompt_renderer,
            host_port,
            progress,
        )
        .await?;
        self.sync_character_profiles_phase(
            chapter,
            workflow_run_id,
            &discovery_result,
            prompt_renderer,
            progress,
            character_trace_port,
        )
        .await?;
        self.refresh_project_status_phase(chapter, workflow_run_id, progress)
            .await?;
        progress.report(100, "章节分析和项目状态同步已完成")
    }

    async fn sync_analysis_outputs_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        if self
            .completed_chapter_analysis_phase(workflow_run_id, chapter, STEP_SYNC_OUTPUTS)
            .await?
            .is_some()
        {
            progress.report(58, "章节分析输出同步已从断点恢复")?;
            return Ok(());
        }
        self.mark_workflow_current_step(workflow_run_id, STEP_SYNC_OUTPUTS)
            .await?;
        self.sync_chapter_analysis_foreshadowing_outputs(chapter)
            .await
            .map_err(|error| AppError::InvalidState {
                current: "chapter_analysis_outputs".to_string(),
                operation: error.to_string(),
            })?;
        host_port
            .sync_chapter_rag_records(chapter, workflow_run_id)
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            STEP_SYNC_OUTPUTS,
            10,
            json!({ "completed": true }),
        )
        .await
    }

    async fn sync_object_discovery_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        if self
            .completed_chapter_analysis_phase(workflow_run_id, chapter, STEP_OBJECT_DISCOVERY)
            .await?
            .is_some()
        {
            progress.report(68, "物体档案发现已从断点恢复")?;
            return Ok(());
        }
        progress.report(58, "正在识别本章关键物体并同步物体档案")?;
        self.mark_workflow_current_step(workflow_run_id, STEP_OBJECT_DISCOVERY)
            .await?;
        let reporter = |current: u32, total: u32, message: &str| {
            progress.subprogress(current, total, 58, 68, message)
        };
        let result = self
            .discover_object_profiles_validated(
                ObjectDiscoveryInput {
                    project_id: chapter.project_id,
                    chapter_number: Some(chapter.chapter_number),
                },
                prompt_renderer,
                Some(ObjectDiscoveryProgress {
                    reporter: Some(&reporter),
                    cancel_token: progress.cancel_token,
                }),
            )
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            STEP_OBJECT_DISCOVERY,
            20,
            json!({ "completed": true, "result": result }),
        )
        .await
    }

    async fn sync_character_discovery_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<Value, AppError> {
        if let Some(checkpoint) = self
            .completed_chapter_analysis_phase(workflow_run_id, chapter, STEP_CHARACTER_DISCOVERY)
            .await?
        {
            progress.report(78, "角色发现已从断点恢复")?;
            return required_checkpoint_field(&checkpoint, STEP_CHARACTER_DISCOVERY, "result")
                .map_err(workflow_contract_error);
        }
        progress.report(68, "正在发现本章新出场角色")?;
        self.mark_workflow_current_step(workflow_run_id, STEP_CHARACTER_DISCOVERY)
            .await?;
        let reporter = |current: u32, total: u32, message: &str| {
            progress.subprogress(current, total, 68, 78, message)
        };
        let result = self
            .discover_character_profiles_validated(
                CharacterDiscoveryInput {
                    project_id: chapter.project_id,
                    chapter_number: Some(chapter.chapter_number),
                },
                prompt_renderer,
                Some(CharacterDiscoveryProgress {
                    reporter: Some(&reporter),
                    cancel_token: progress.cancel_token,
                }),
            )
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            STEP_CHARACTER_DISCOVERY,
            30,
            json!({ "completed": true, "result": result.clone() }),
        )
        .await?;
        Ok(result)
    }

    async fn sync_scene_profiles_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        if self
            .completed_chapter_analysis_phase(workflow_run_id, chapter, STEP_SCENE_SYNC)
            .await?
            .is_some()
        {
            progress.report(88, "场景档案同步已从断点恢复")?;
            return Ok(());
        }
        progress.report(78, "正在同步本章场景档案和场景内状态")?;
        self.mark_workflow_current_step(workflow_run_id, STEP_SCENE_SYNC)
            .await?;
        let reporter = |current: u32, total: u32, message: &str| {
            let _ = progress.subprogress(current, total, 78, 88, message);
        };
        let scene_progress = SceneProfileSyncProgress::new(Some(&reporter), progress.cancel_token);
        self.sync_scene_profiles_for_chapters_validated(
            chapter.project_id,
            Some(chapter.chapter_number),
            prompt_renderer,
            host_port.scene_profile_sync_host_port(),
            Some(&scene_progress),
        )
        .await
        .map_err(|error| AppError::InvalidState {
            current: "chapter_analysis_scene_sync".to_string(),
            operation: error.to_string(),
        })?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            STEP_SCENE_SYNC,
            40,
            json!({ "completed": true }),
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn sync_character_profiles_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        discovery_result: &Value,
        prompt_renderer: &dyn PromptRenderer,
        progress: ChapterAnalysisApplicationProgress<'_>,
        trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError> {
        if self
            .completed_chapter_analysis_phase(workflow_run_id, chapter, STEP_CHARACTER_SYNC)
            .await?
            .is_some()
        {
            progress.report(97, "角色档案同步已从断点恢复")?;
            return Ok(());
        }
        progress.report(88, "正在同步角色档案、事件和状态索引")?;
        self.mark_workflow_current_step(workflow_run_id, STEP_CHARACTER_SYNC)
            .await?;
        let reporter = |current: u32, total: u32, message: &str| {
            progress.subprogress(current, total, 88, 97, message)
        };
        let sync_result = self
            .run_analysis_character_profile_sync_validated(
                chapter.project_id,
                chapter.chapter_number,
                chapter.analysis_data.as_ref(),
                Some(discovery_result),
                prompt_renderer,
                CharacterProfileBatchProgress {
                    reporter: Some(&reporter),
                },
                trace_port,
            )
            .await?;
        ensure_chapter_analysis_character_sync_completed(&sync_result).map_err(|error| {
            AppError::InvalidState {
                current: "chapter_analysis_character_sync".to_string(),
                operation: error.to_string(),
            }
        })?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            STEP_CHARACTER_SYNC,
            50,
            json!({ "completed": true, "result": sync_result }),
        )
        .await
    }

    async fn refresh_project_status_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        if self
            .completed_chapter_analysis_phase(workflow_run_id, chapter, STEP_REFRESH_STATUS)
            .await?
            .is_some()
        {
            progress.report(99, "项目进度状态刷新已从断点恢复")?;
            return Ok(());
        }
        progress.report(97, "正在刷新项目进度状态")?;
        self.mark_workflow_current_step(workflow_run_id, STEP_REFRESH_STATUS)
            .await?;
        self.refresh_project_current_chapter(chapter.project_id)
            .await?;
        self.sync_project_status_from_artifacts(chapter.project_id)
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            STEP_REFRESH_STATUS,
            60,
            json!({ "completed": true }),
        )
        .await
    }

    async fn completed_chapter_analysis_phase(
        &self,
        workflow_run_id: i64,
        chapter: &chapter::Model,
        step_key: &str,
    ) -> Result<Option<Value>, AppError> {
        let fingerprint = chapter_analysis_phase_fingerprint(chapter, step_key);
        self.load_completed_workflow_checkpoint(Some(workflow_run_id), step_key, &fingerprint)
            .await?
            .map(|checkpoint| {
                let json = required_checkpoint_json(&checkpoint, step_key)
                    .map_err(workflow_contract_error)?;
                require_completed_chapter_analysis_phase(&json, step_key)
                    .map_err(workflow_contract_error)?;
                Ok(json)
            })
            .transpose()
    }

    async fn save_chapter_analysis_phase(
        &self,
        workflow_run_id: i64,
        chapter: &chapter::Model,
        step_key: &str,
        step_order: i32,
        checkpoint_json: Value,
    ) -> Result<(), AppError> {
        self.save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow_run_id),
            step_key: step_key.to_string(),
            step_order,
            input_fingerprint_json: chapter_analysis_phase_fingerprint(chapter, step_key),
            checkpoint_json,
        })
        .await
    }

    async fn ensure_previous_chapter_analysis_complete(
        &self,
        chapter: &chapter::Model,
        operation: &str,
    ) -> Result<(), AppError> {
        let previous = self
            .chapters_before_number(chapter.project_id, chapter.chapter_number)
            .await?;
        ensure_previous_chapter_analysis_complete_from_rows(
            operation,
            chapter.chapter_number,
            &previous,
        )
        .map_err(|error| AppError::InvalidState {
            current: "previous_chapter_analysis_incomplete".to_string(),
            operation: error.to_string(),
        })
    }

    async fn ensure_chapter_analysis_sync_resume_needed(
        &self,
        chapter: &chapter::Model,
    ) -> Result<(), AppError> {
        let run = self
            .latest_workflow_run_by_business_key(&chapter_analysis_workflow_business_key(
                chapter.project_id,
                chapter.id,
            ))
            .await?;
        if let Some(run) = run
            && let Some(reason) = chapter_analysis_resume_block_reason(&run, chapter)
        {
            return Err(AppError::InvalidState {
                current: run.status,
                operation: reason.to_string(),
            });
        }
        Ok(())
    }

    async fn finalize_chapter_analysis_application<T>(
        &self,
        workflow_run_id: i64,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        let outcome = match &result {
            Ok(_) => WorkflowRunFinalizeOutcome::Completed,
            Err(AppError::Cancelled) => WorkflowRunFinalizeOutcome::Cancelled,
            Err(_) => WorkflowRunFinalizeOutcome::Failed,
        };
        let error_message = result.as_ref().err().map(ToString::to_string);
        let finalized = self
            .finalize_workflow_run_outcome_validated(
                workflow_run_id,
                outcome,
                error_message.as_deref(),
            )
            .await;
        match (result, finalized) {
            (Ok(value), Ok(_)) => Ok(value),
            (Err(error), Ok(_)) => Err(error),
            (Ok(_), Err(error)) => Err(AppError::InvalidState {
                current: "chapter_analysis_completed".to_string(),
                operation: format!("章节分析完成，但工作流终态写入失败: {error}"),
            }),
            (Err(error), Err(finalize_error)) => Err(AppError::InvalidState {
                current: error.to_string(),
                operation: format!("章节分析失败，且工作流终态写入失败: {finalize_error}"),
            }),
        }
    }
}

fn ensure_chapter_not_generating(chapter: &chapter::Model) -> Result<(), AppError> {
    if chapter.is_generating() {
        Err(AppError::InvalidState {
            current: "chapter_generating".to_string(),
            operation: format!("处理第 {} 章结构化分析", chapter.chapter_number),
        })
    } else {
        Ok(())
    }
}

fn workflow_contract_error(error: impl ToString) -> AppError {
    AppError::InvalidState {
        current: "chapter_analysis_checkpoint".to_string(),
        operation: error.to_string(),
    }
}

fn blocked_preview_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<&str>,
    note: impl Into<String>,
) -> ChapterAnalysisPreviewStep {
    ChapterAnalysisPreviewStep {
        key: key.into(),
        title: title.into(),
        prompt_name: prompt_name.map(str::to_string),
        status: "blocked".to_string(),
        note: Some(note.into()),
        request: None,
    }
}

fn blocked_scene_preview_steps(
    chapter_number: i32,
    first_note: &str,
) -> Vec<ChapterAnalysisPreviewStep> {
    vec![
        blocked_preview_step(
            format!("scene_profile_sync_ch{chapter_number}_scenes"),
            "分析后场景档案候选识别请求",
            Some("scene_profile_candidates"),
            first_note,
        ),
        blocked_preview_step(
            format!("scene_profile_sync_ch{chapter_number}_occurrences"),
            "分析后场景出场抽取请求",
            Some("scene_occurrence_extraction"),
            "该请求依赖场景档案候选识别返回的 scenes，发送前无法提前构造真实请求",
        ),
        blocked_preview_step(
            format!("scene_profile_sync_ch{chapter_number}_transitions"),
            "分析后场景转场抽取请求",
            Some("scene_transition_extraction"),
            "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求",
        ),
        blocked_preview_step(
            format!("scene_profile_sync_ch{chapter_number}_entity_states"),
            "分析后场景实体状态抽取请求",
            Some("scene_entity_state_extraction"),
            "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_scene_preview_covers_full_scene_sync_chain() {
        let steps = blocked_scene_preview_steps(3, "结构化分析保存后才能构造");
        assert_eq!(steps.len(), 4);
        assert!(steps.iter().all(|step| step.status == "blocked"));
        assert_eq!(steps[0].key, "scene_profile_sync_ch3_scenes");
        assert_eq!(
            steps
                .iter()
                .filter_map(|step| step.prompt_name.as_deref())
                .collect::<Vec<_>>(),
            vec![
                "scene_profile_candidates",
                "scene_occurrence_extraction",
                "scene_transition_extraction",
                "scene_entity_state_extraction",
            ]
        );
    }
}

use a4n_api_contract::chapter::{
    CHAPTER_ANALYSIS_PROGRESS_TOTAL, ChapterAnalysisMutationResult, ChapterDerivedCleanupHostPort,
    ChapterSummary,
};
use a4n_api_contract::character::DiscoverCharacterProfilesInput;
use a4n_chapter_analysis::validate_saved_chapter_analysis;
use a4n_db_models::models::{
    chapter::chapter,
    generation::{workflow_checkpoint, workflow_run},
};
use a4n_domain::project::ChapterDerivedCleanupArtifacts;
use a4n_error::AppError;
use a4n_prompt::{chapter_analysis::ChapterAnalysisRequestStep, rendering::PromptRenderer};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    character::{
        CharacterDiscoveryProgress, CharacterProfileSyncProgress,
        CharacterProfileSyncStageTracePort,
    },
    object_discovery::{
        OBJECT_DISCOVERY_CALL_KEY, OBJECT_DISCOVERY_KIND, ObjectDiscoveryInput,
        ObjectDiscoveryProgress,
    },
    scene_profile::{SceneProfileSyncHostPort, SceneProfileSyncProgress},
    workflow::{
        CompletedWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, required_checkpoint_field,
        required_checkpoint_json, workflow_checkpoint_fingerprint_sha256,
    },
};

use super::outputs::{
    ensure_chapter_analysis_character_sync_completed, retrospective_chapter_outline_input,
};
use super::{
    CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
    ChapterAnalysisInput, build_chapter_analysis_initial_request,
    chapter_analysis_phase_fingerprint, chapter_analysis_resume_block_reason,
    chapter_analysis_run_input_matches, chapter_analysis_run_uses_replacement_commit,
    chapter_analysis_workflow_business_key, require_completed_chapter_analysis_phase,
};

const STEP_FACTS: &str = "chapter_analysis.facts";
const STEP_DERIVED_SYNC: &str = "chapter_analysis.derived_sync";
pub const CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS: &str = "chapter_analysis.sync_outputs";
pub const CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC: &str = "chapter_analysis.outline_sync";
pub const CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY: &str = "chapter_analysis.object_discovery";
pub const CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY: &str = "chapter_analysis.character_discovery";
pub const CHAPTER_ANALYSIS_STEP_SCENE_SYNC: &str = "chapter_analysis.scene_sync";
pub const CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC: &str = "chapter_analysis.character_sync";
const STEP_REFRESH_STATUS: &str = "chapter_analysis.refresh_status";

pub type ChapterAnalysisApplicationProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);

#[derive(Clone, Copy, Default)]
pub struct ChapterAnalysisApplicationProgress<'a> {
    pub reporter: Option<ChapterAnalysisApplicationProgressReporter<'a>>,
    /// 活体信号出口：与字段级里程碑取自同一个 token 流。
    pub stream_reporter: Option<&'a (dyn Fn(&str) + Send + Sync)>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub active_task_id: Option<&'a str>,
}

impl ChapterAnalysisApplicationProgress<'_> {
    fn report(&self, current: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter {
            reporter(current, CHAPTER_ANALYSIS_PROGRESS_TOTAL, message)?;
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
            reporter(mapped, CHAPTER_ANALYSIS_PROGRESS_TOTAL, message)?;
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
    fn scene_profile_sync_host_port(&self) -> &dyn SceneProfileSyncHostPort;
}

#[derive(Debug)]
pub struct ChapterAnalysisRequestPreview {
    pub title: String,
    pub steps: Vec<ChapterAnalysisRequestStep>,
}

/// Runtime-facing boundary for a new chapter-analysis run.
///
/// Analysis validation, authorization, workflow state and API projections stay
/// in the chapter-analysis owner. Runtime supplies only prompt, cleanup and
/// progress adapters.
#[async_trait]
pub trait ChapterAnalysisGenerationApplicationPort: Send + Sync {
    async fn preflight_generate(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError>;

    async fn generate(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterAnalysisMutationResult, AppError>;
}

/// Runtime-facing boundary for chapter-analysis synchronization and workflow
/// recovery.
#[async_trait]
pub trait ChapterAnalysisResumeApplicationPort: Send + Sync {
    async fn preflight_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError>;

    async fn resume_sync(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterSummary, AppError>;

    async fn resume_workflow_run(
        &self,
        user_id: i64,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError>;
}

/// Runtime-facing boundary for new chapter-analysis request previews.
#[async_trait]
pub trait ChapterAnalysisGenerationPreviewApplicationPort: Send + Sync {
    async fn preview(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError>;
}

/// Runtime-facing boundary for chapter-analysis resume request previews.
#[async_trait]
pub trait ChapterAnalysisResumePreviewApplicationPort: Send + Sync {
    async fn preview_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError>;
}

impl OrchestratorContext {
    pub async fn prepare_resume_chapter_analysis_preview_validated(
        &self,
        chapter: &chapter::Model,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
        let run = self.resumable_chapter_analysis_run(chapter).await?;
        if !self
            .chapter_analysis_run_has_saved_analysis(&run, chapter)
            .await?
        {
            return Ok(vec![
                build_chapter_analysis_initial_request(chapter, prompt_renderer).await?,
            ]);
        }

        if self
            .completed_chapter_analysis_phase(
                run.id,
                chapter,
                CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
            )
            .await?
            .is_none()
        {
            let request = self
                .prepare_object_discovery_request_preview_validated(
                    ObjectDiscoveryInput {
                        project_id: chapter.project_id,
                        chapter_number: Some(chapter.chapter_number),
                    },
                    prompt_renderer,
                )
                .await?;
            return Ok(vec![ChapterAnalysisRequestStep {
                key: OBJECT_DISCOVERY_CALL_KEY.to_string(),
                title: "物体档案发现初始 LLM 请求".to_string(),
                prompt_name: OBJECT_DISCOVERY_KIND.to_string(),
                request,
            }]);
        }

        if self
            .completed_chapter_analysis_phase(
                run.id,
                chapter,
                CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            )
            .await?
            .is_none()
        {
            let preview = self
                .prepare_character_discovery_request_preview_validated(
                    DiscoverCharacterProfilesInput {
                        project_id: chapter.project_id,
                        chapter_number: Some(chapter.chapter_number),
                    },
                    prompt_renderer,
                )
                .await?;
            return Ok(vec![ChapterAnalysisRequestStep {
                key: crate::character::CHARACTER_DISCOVERY_CALL_KEY.to_string(),
                title: "角色发现初始 LLM 请求".to_string(),
                prompt_name: crate::character::CHARACTER_DISCOVERY_KIND.to_string(),
                request: preview.request,
            }]);
        }

        if self
            .completed_chapter_analysis_phase(run.id, chapter, CHAPTER_ANALYSIS_STEP_SCENE_SYNC)
            .await?
            .is_none()
        {
            return self
                .prepare_scene_profile_sync_preview_validated(
                    chapter.project_id,
                    Some(chapter.chapter_number),
                    prompt_renderer,
                )
                .await;
        }

        Ok(Vec::new())
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
        Ok(chapter)
    }

    pub async fn preflight_resume_chapter_analysis_validated(
        &self,
        chapter_id: i64,
    ) -> Result<chapter::Model, AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.resumable_chapter_analysis_run(&chapter).await?;
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
            None,
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
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        let resumable = self.resumable_chapter_analysis_run(&chapter).await?;
        let has_saved_analysis = self
            .chapter_analysis_run_has_saved_analysis(&resumable, &chapter)
            .await?;
        let workflow_run = self.mark_workflow_running_validated(resumable.id).await?;
        if !has_saved_analysis {
            return self
                .run_chapter_analysis_inner(
                    chapter_id,
                    Some(workflow_run),
                    prompt_renderer,
                    host_port,
                    progress,
                    character_trace_port,
                )
                .await;
        }
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
        self.finalize_workflow_result(workflow_run.id, result, "章节分析")
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
        crate::workflow::require_workflow_can_resume(
            &run.status,
            run.can_resume,
            run.resume_block_reason.as_deref(),
        )
        .map_err(|operation| AppError::InvalidState {
            current: run.status.clone(),
            operation,
        })?;
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
        let latest = self
            .latest_workflow_run_by_business_key(&chapter_analysis_workflow_business_key(
                chapter.project_id,
                chapter.id,
            ))
            .await?
            .ok_or_else(|| AppError::NotFound("章节结构化分析工作流不存在".to_string()))?;
        if latest.id != run.id {
            return Err(AppError::InvalidState {
                current: "chapter_analysis_workflow_replaced".to_string(),
                operation: "恢复已被更新运行取代的章节分析".to_string(),
            });
        }
        let has_saved_analysis = self
            .chapter_analysis_run_has_saved_analysis(&latest, &chapter)
            .await?;
        let running = self.mark_workflow_running_validated(run.id).await?;
        if has_saved_analysis {
            let result = self
                .sync_project_state_after_chapter_analysis(
                    &chapter,
                    running.id,
                    prompt_renderer,
                    host_port,
                    ChapterAnalysisApplicationProgress::default(),
                    character_trace_port,
                )
                .await;
            self.finalize_workflow_result(running.id, result, "章节分析")
                .await?;
        } else {
            self.run_chapter_analysis_inner(
                chapter_id,
                Some(running),
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
        resumed_workflow_run: Option<workflow_run::Model>,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<chapter::Model, AppError> {
        progress.report(0, "正在检查章节正文")?;
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;

        progress.report(10, "正在读取章节并校验角色同步前置条件")?;
        let workflow_run = match resumed_workflow_run {
            Some(run) => {
                if run.status != crate::workflow::WORKFLOW_STATUS_RUNNING
                    || !chapter_analysis_run_input_matches(&run.input_fingerprint_json, &chapter)
                {
                    return Err(AppError::InvalidState {
                        current: "chapter_analysis_resume_source_changed".to_string(),
                        operation: "恢复章节结构化分析".to_string(),
                    });
                }
                run
            }
            None => {
                self.begin_chapter_analysis_workflow_run(
                    &chapter,
                    STEP_FACTS,
                    progress.active_task_id,
                )
                .await?
            }
        };
        progress.report(20, "正在调用模型生成结构化分析")?;
        let analysis_reporter = |current: u32, total: u32, message: &str| {
            progress.subprogress(current, total, 20, 45, message)
        };
        let mut mapped_analysis_progress =
            |current: u32, total: u32, message: &str| analysis_reporter(current, total, message);
        let result = async {
            let updated = self
                .analyze_and_store_chapter(
                    ChapterAnalysisInput {
                        chapter,
                        prompt_renderer,
                        progress: Some(&mut mapped_analysis_progress),
                        stream_reporter: progress.stream_reporter,
                        cancel_token: progress.cancel_token,
                        workflow_run_id: Some(workflow_run.id),
                    },
                    host_port,
                )
                .await?;
            progress.report(45, "结构化分析已保存，正在读取章节")?;
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
        self.finalize_workflow_result(workflow_run.id, result, "章节分析")
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
        self.finish_chapter_analysis_replacement_cleanup(
            chapter,
            workflow_run_id,
            host_port,
            progress,
        )
        .await?;
        self.mark_workflow_current_step(workflow_run_id, STEP_DERIVED_SYNC)
            .await?;
        progress.report(50, "正在同步本章章纲")?;
        self.sync_retrospective_outline_phase(chapter, workflow_run_id, progress)
            .await?;
        progress.report(54, "正在同步章节分析、伏笔和 RAG")?;
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

    async fn sync_retrospective_outline_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        if self
            .completed_chapter_analysis_phase(
                workflow_run_id,
                chapter,
                CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC,
            )
            .await?
            .is_some()
        {
            progress.report(54, "本章章纲同步已从断点恢复")?;
            return Ok(());
        }
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC)
            .await?;
        let input = retrospective_chapter_outline_input(
            chapter,
            chapter_analysis_phase_fingerprint(chapter, CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC),
        )
        .map_err(|error| AppError::InvalidState {
            current: "chapter_analysis_outline".to_string(),
            operation: error.to_string(),
        })?;
        let outcome = a4n_db::project::upsert_retrospective_chapter_outline_checked(
            self.database.connection(),
            input,
            a4n_db::project::ChapterSourceSnapshot::from(chapter),
        )
        .await
        .map_err(crate::project::project_outline_record_error_to_app_error)?;
        let name = outcome
            .outline
            .title
            .clone()
            .unwrap_or_else(|| format!("第{}章章纲", chapter.chapter_number));
        let result = match outcome.action {
            a4n_db::project::RetrospectiveChapterOutlineAction::Created => {
                json!({ "created": [{ "name": name, "outline_id": outcome.outline.id }] })
            }
            a4n_db::project::RetrospectiveChapterOutlineAction::Updated => {
                json!({ "updated": [{ "name": name, "outline_id": outcome.outline.id }] })
            }
            a4n_db::project::RetrospectiveChapterOutlineAction::Unchanged => {
                json!({ "existing": [{ "name": name, "outline_id": outcome.outline.id }] })
            }
            a4n_db::project::RetrospectiveChapterOutlineAction::Preserved => {
                json!({ "preserved": [{ "name": name, "outline_id": outcome.outline.id }] })
            }
        };
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            CHAPTER_ANALYSIS_STEP_OUTLINE_SYNC,
            8,
            json!({
                "completed": true,
                "result": result
            }),
        )
        .await
    }

    async fn sync_analysis_outputs_phase(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        if self
            .completed_chapter_analysis_phase(
                workflow_run_id,
                chapter,
                CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
            )
            .await?
            .is_some()
        {
            progress.report(58, "章节分析输出同步已从断点恢复")?;
            return Ok(());
        }
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS)
            .await?;
        let foreshadowing_result = self
            .sync_chapter_analysis_foreshadowing_outputs(chapter)
            .await
            .map_err(|error| AppError::InvalidState {
                current: "chapter_analysis_outputs".to_string(),
                operation: error.to_string(),
            })?;
        host_port
            .sync_chapter_rag_records(chapter.project_id, chapter.id, workflow_run_id)
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS,
            10,
            json!({
                "completed": true,
                "result": { "foreshadowing": foreshadowing_result }
            }),
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
            .completed_chapter_analysis_phase(
                workflow_run_id,
                chapter,
                CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
            )
            .await?
            .is_some()
        {
            progress.report(68, "物体档案发现已从断点恢复")?;
            return Ok(());
        }
        progress.report(58, "正在识别本章关键物体并同步物体档案")?;
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY)
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
                    // 嵌套子步骤沿用同一活体信号出口。
                    stream_reporter: progress.stream_reporter,
                    cancel_token: progress.cancel_token,
                    active_task_id: progress.active_task_id,
                }),
            )
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY,
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
            .completed_chapter_analysis_phase(
                workflow_run_id,
                chapter,
                CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            )
            .await?
        {
            progress.report(78, "角色发现已从断点恢复")?;
            return required_checkpoint_field(
                &checkpoint,
                CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
                "result",
            )
            .map_err(workflow_contract_error);
        }
        progress.report(68, "正在发现本章新出场角色")?;
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY)
            .await?;
        let reporter = |current: u32, total: u32, message: &str| {
            progress.subprogress(current, total, 68, 78, message)
        };
        let result = self
            .discover_character_profiles_validated(
                DiscoverCharacterProfilesInput {
                    project_id: chapter.project_id,
                    chapter_number: Some(chapter.chapter_number),
                },
                prompt_renderer,
                Some(CharacterDiscoveryProgress {
                    reporter: Some(&reporter),
                    // 嵌套子步骤沿用同一活体信号出口。
                    stream_reporter: progress.stream_reporter,
                    cancel_token: progress.cancel_token,
                    active_task_id: progress.active_task_id,
                }),
            )
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY,
            30,
            json!({ "completed": true, "result": result.clone() }),
        )
        .await?;
        serde_json::to_value(result).map_err(AppError::from)
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
            .completed_chapter_analysis_phase(
                workflow_run_id,
                chapter,
                CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
            )
            .await?
            .is_some()
        {
            progress.report(88, "场景档案同步已从断点恢复")?;
            return Ok(());
        }
        progress.report(78, "正在同步本章场景档案和场景内状态")?;
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_SCENE_SYNC)
            .await?;
        let reporter = |current: u32, total: u32, message: &str| {
            let _ = progress.subprogress(current, total, 78, 88, message);
        };
        let scene_progress = SceneProfileSyncProgress::new(
            Some(&reporter),
            progress.cancel_token,
            progress.active_task_id,
        );
        self.sync_scene_profiles_validated(
            chapter.project_id,
            Some(chapter.chapter_number),
            prompt_renderer,
            host_port.scene_profile_sync_host_port(),
            Some(&scene_progress),
        )
        .await
        .map_err(|error| match error {
            AppError::Cancelled => AppError::Cancelled,
            error => AppError::InvalidState {
                current: "chapter_analysis_scene_sync".to_string(),
                operation: error.to_string(),
            },
        })?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            CHAPTER_ANALYSIS_STEP_SCENE_SYNC,
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
            .completed_chapter_analysis_phase(
                workflow_run_id,
                chapter,
                CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
            )
            .await?
            .is_some()
        {
            progress.report(97, "角色档案同步已从断点恢复")?;
            return Ok(());
        }
        progress.report(88, "正在同步角色档案、事件和状态索引")?;
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC)
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
                CharacterProfileSyncProgress {
                    reporter: Some(&reporter),
                    active_task_id: progress.active_task_id,
                },
                trace_port,
            )
            .await?;
        ensure_chapter_analysis_character_sync_completed(&sync_result.batch.failed)
            .map_err(|error| AppError::LlmError(error.to_string()))?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC,
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

    async fn finish_chapter_analysis_replacement_cleanup(
        &self,
        chapter: &chapter::Model,
        workflow_run_id: i64,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        if !chapter_analysis_run_uses_replacement_commit(&run.input_fingerprint_json) {
            return Ok(());
        }
        if self
            .completed_chapter_analysis_phase(
                workflow_run_id,
                chapter,
                CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP,
            )
            .await?
            .is_some()
        {
            progress.report(50, "旧章节派生资料清理已从断点恢复")?;
            return Ok(());
        }

        let artifacts = self
            .chapter_analysis_replacement_cleanup_artifacts(&run, chapter)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_chapter_analysis_replacement_commit".to_string(),
                operation: "恢复章节分析替换后的外部清理".to_string(),
            })?;
        host_port.validate_chapter_cleanup_artifacts(&artifacts)?;
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP)
            .await?;
        progress.report(47, "正在清理旧章节分析对应的外部资料")?;
        host_port
            .finish_chapter_derived_cleanup(
                chapter.project_id,
                chapter.id,
                chapter.chapter_number,
                artifacts,
            )
            .await?;
        self.save_chapter_analysis_phase(
            workflow_run_id,
            chapter,
            CHAPTER_ANALYSIS_REPLACEMENT_CLEANUP_STEP,
            7,
            json!({ "completed": true }),
        )
        .await
    }

    pub async fn chapter_analysis_run_has_saved_analysis(
        &self,
        run: &workflow_run::Model,
        chapter: &chapter::Model,
    ) -> Result<bool, AppError> {
        if run.workflow_kind != "chapter_analysis"
            || run.project_id != Some(chapter.project_id)
            || run.chapter_id != Some(chapter.id)
            || !chapter_analysis_run_input_matches(&run.input_fingerprint_json, chapter)
        {
            return Ok(false);
        }
        let checkpoint =
            if chapter_analysis_run_uses_replacement_commit(&run.input_fingerprint_json) {
                let fingerprint = chapter_analysis_phase_fingerprint(
                    chapter,
                    CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
                );
                self.load_completed_workflow_checkpoint(
                    Some(run.id),
                    CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
                    &fingerprint,
                )
                .await?
            } else {
                None
            };
        self.chapter_analysis_run_has_saved_analysis_from_checkpoint(
            run,
            chapter,
            checkpoint.as_ref(),
        )
    }

    /// 使用批量读取的 checkpoint 判断章节分析是否已经保存完整结果。
    pub fn chapter_analysis_run_has_saved_analysis_from_checkpoints(
        &self,
        run: &workflow_run::Model,
        chapter: &chapter::Model,
        checkpoints: &[workflow_checkpoint::Model],
    ) -> Result<bool, AppError> {
        if run.workflow_kind != "chapter_analysis"
            || run.project_id != Some(chapter.project_id)
            || run.chapter_id != Some(chapter.id)
            || !chapter_analysis_run_input_matches(&run.input_fingerprint_json, chapter)
        {
            return Ok(false);
        }
        if !chapter_analysis_run_uses_replacement_commit(&run.input_fingerprint_json) {
            return self
                .chapter_analysis_run_has_saved_analysis_from_checkpoint(run, chapter, None);
        }
        let fingerprint =
            chapter_analysis_phase_fingerprint(chapter, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP);
        let fingerprint_sha256 = workflow_checkpoint_fingerprint_sha256(&fingerprint)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let checkpoint = checkpoints.iter().find(|checkpoint| {
            checkpoint.step_key == CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP
                && checkpoint.status == WORKFLOW_STATUS_COMPLETED
                && checkpoint.input_fingerprint_sha256.as_deref() == Some(&fingerprint_sha256)
        });
        self.chapter_analysis_run_has_saved_analysis_from_checkpoint(run, chapter, checkpoint)
    }

    fn chapter_analysis_run_has_saved_analysis_from_checkpoint(
        &self,
        run: &workflow_run::Model,
        chapter: &chapter::Model,
        checkpoint: Option<&workflow_checkpoint::Model>,
    ) -> Result<bool, AppError> {
        if chapter_analysis_run_uses_replacement_commit(&run.input_fingerprint_json) {
            let Some(checkpoint) = checkpoint else {
                return Ok(false);
            };
            let checkpoint_json =
                required_checkpoint_json(checkpoint, CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP)
                    .map_err(workflow_contract_error)?;
            require_completed_chapter_analysis_phase(
                &checkpoint_json,
                CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
            )
            .map_err(workflow_contract_error)?;
            replacement_cleanup_artifacts_from_checkpoint(&checkpoint_json)?;
        }
        let Some(analysis) = chapter.analysis_data.as_ref() else {
            return Ok(false);
        };
        validate_saved_chapter_analysis(analysis).map_err(|error| AppError::InvalidState {
            current: format!("invalid_saved_chapter_analysis: {error}"),
            operation: "读取章节分析工作流保存状态".to_string(),
        })?;
        Ok(true)
    }

    async fn chapter_analysis_replacement_cleanup_artifacts(
        &self,
        run: &workflow_run::Model,
        chapter: &chapter::Model,
    ) -> Result<Option<ChapterDerivedCleanupArtifacts>, AppError> {
        if !chapter_analysis_run_uses_replacement_commit(&run.input_fingerprint_json) {
            return Ok(None);
        }
        let Some(checkpoint) = self
            .completed_chapter_analysis_phase(
                run.id,
                chapter,
                CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
            )
            .await?
        else {
            return Ok(None);
        };
        replacement_cleanup_artifacts_from_checkpoint(&checkpoint).map(Some)
    }

    async fn resumable_chapter_analysis_run(
        &self,
        chapter: &chapter::Model,
    ) -> Result<workflow_run::Model, AppError> {
        let run = self
            .latest_workflow_run_by_business_key(&chapter_analysis_workflow_business_key(
                chapter.project_id,
                chapter.id,
            ))
            .await?
            .ok_or_else(|| AppError::NotFound("没有可恢复的章节结构化分析".to_string()))?;
        if !chapter_analysis_run_input_matches(&run.input_fingerprint_json, chapter) {
            return Err(AppError::InvalidState {
                current: "chapter_analysis_input_changed".to_string(),
                operation: "恢复章节结构化分析".to_string(),
            });
        }
        if let Some(reason) = chapter_analysis_resume_block_reason(&run, chapter) {
            return Err(AppError::InvalidState {
                current: run.status.clone(),
                operation: reason.to_string(),
            });
        }
        crate::workflow::require_workflow_can_resume(
            &run.status,
            run.can_resume,
            run.resume_block_reason.as_deref(),
        )
        .map_err(|operation| AppError::InvalidState {
            current: run.status.clone(),
            operation,
        })?;
        Ok(run)
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
            checkpoint_json: Some(checkpoint_json),
        })
        .await
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
}

#[async_trait]
impl ChapterAnalysisGenerationApplicationPort for OrchestratorContext {
    async fn preflight_generate(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.chapter_summary_for_user(user_id, chapter.id).await
    }

    async fn generate(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterAnalysisMutationResult, AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        match self
            .run_chapter_analysis_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                progress,
                character_trace_port,
            )
            .await
        {
            Ok(chapter) => Ok(ChapterAnalysisMutationResult {
                chapter: self.chapter_summary_for_user(user_id, chapter.id).await?,
                warnings: Vec::new(),
            }),
            Err(sync_error) => {
                let summary = self.chapter_summary_for_user(user_id, chapter_id).await?;
                if !summary.analysis_sync_has_saved_analysis || !summary.analysis_sync_can_resume {
                    return Err(sync_error);
                }
                Ok(ChapterAnalysisMutationResult {
                    chapter: summary,
                    warnings: vec![format!(
                        "结构化分析已保存，但后续资料同步未完成：{sync_error}"
                    )],
                })
            }
        }
    }
}

#[async_trait]
impl ChapterAnalysisResumeApplicationPort for OrchestratorContext {
    async fn preflight_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<ChapterSummary, AppError> {
        let chapter = self
            .preflight_resume_chapter_analysis_validated(chapter_id)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.chapter_summary_for_user(user_id, chapter.id).await
    }

    async fn resume_sync(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<ChapterSummary, AppError> {
        let chapter = self
            .preflight_resume_chapter_analysis_validated(chapter_id)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let chapter = self
            .resume_chapter_analysis_sync_validated(
                chapter_id,
                prompt_renderer,
                host_port,
                progress,
                character_trace_port,
            )
            .await?;
        self.chapter_summary_for_user(user_id, chapter.id).await
    }

    async fn resume_workflow_run(
        &self,
        user_id: i64,
        workflow_run_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        character_trace_port: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError> {
        let run = self.workflow_run_model_by_id(workflow_run_id).await?;
        let chapter_id = run.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument("章节结构化分析工作流缺少 chapter_id".to_string())
        })?;
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.resume_chapter_analysis_workflow_run_validated(
            &run,
            prompt_renderer,
            host_port,
            character_trace_port,
        )
        .await
    }
}

#[async_trait]
impl ChapterAnalysisGenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError> {
        let chapter = self
            .preflight_chapter_analysis_validated(chapter_id, false)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        let step = build_chapter_analysis_initial_request(&chapter, prompt_renderer).await?;
        Ok(ChapterAnalysisRequestPreview {
            title: format!("第 {} 章结构化分析请求预览", chapter.chapter_number),
            steps: vec![step],
        })
    }
}

#[async_trait]
impl ChapterAnalysisResumePreviewApplicationPort for OrchestratorContext {
    async fn preview_resume(
        &self,
        user_id: i64,
        chapter_id: i64,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterAnalysisRequestPreview, AppError> {
        let chapter = self
            .preflight_resume_chapter_analysis_validated(chapter_id)
            .await?;
        self.verify_project_permission(
            chapter.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(ChapterAnalysisRequestPreview {
            title: format!("第 {} 章分析继续同步请求预览", chapter.chapter_number),
            steps: self
                .prepare_resume_chapter_analysis_preview_validated(&chapter, prompt_renderer)
                .await?,
        })
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

fn replacement_cleanup_artifacts_from_checkpoint(
    checkpoint: &Value,
) -> Result<ChapterDerivedCleanupArtifacts, AppError> {
    let artifacts = required_checkpoint_field(
        checkpoint,
        CHAPTER_ANALYSIS_REPLACEMENT_COMMIT_STEP,
        "cleanup_artifacts",
    )
    .map_err(workflow_contract_error)?;
    serde_json::from_value(artifacts).map_err(|error| AppError::InvalidState {
        current: "chapter_analysis_replacement_commit".to_string(),
        operation: format!("章节分析替换清理产物合同无效: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_commit_cleanup_artifacts_round_trip() {
        let checkpoint = json!({
            "completed": true,
            "cleanup_artifacts": {
                "stale_profile_ids": [7],
                "media_file_paths": ["projects/1/old.png"],
                "scene_records_changed": true
            }
        });

        let artifacts = replacement_cleanup_artifacts_from_checkpoint(&checkpoint)
            .expect("valid replacement cleanup artifacts should restore");

        assert_eq!(artifacts.stale_profile_ids, vec![7]);
        assert_eq!(artifacts.media_file_paths, vec!["projects/1/old.png"]);
        assert!(artifacts.scene_records_changed);
    }

    #[test]
    fn replacement_commit_requires_cleanup_artifacts() {
        let error = replacement_cleanup_artifacts_from_checkpoint(&json!({"completed": true}))
            .expect_err("missing replacement cleanup artifacts must fail explicitly");

        assert!(error.to_string().contains("cleanup_artifacts"));
    }
}

use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
    optimize::{
        AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        CreateOptimizationChatSessionInput, ListOptimizeSessionsInput, OptimizationPatchSetSummary,
        OptimizationSessionSummary, OptimizationTurnSummary, OptimizeSessionInput,
        PrepareOptimizationAnalysisSessionInput, RecordOptimizeAgentEventInput,
        SendOptimizationMessageInput, StartOptimizeAnalysisGenerationInput,
        StartOptimizeChatGenerationInput, StartOptimizePatchGenerationInput,
    },
};
use a4n_db::optimize::OptimizationChapterPatchWriteInput;
use a4n_db_models::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_turn,
    optimization_turn_event,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_optimize_host_contract::{
    OptimizeAnalysisGenerationApplicationPort, OptimizeAnalysisPreviewApplicationPort,
    OptimizeChatGenerationApplicationPort, OptimizeChatPreviewApplicationPort,
    OptimizePatchApplicationPort, OptimizePatchApplyHostPort,
    OptimizePatchGenerationApplicationPort, OptimizePatchPreviewApplicationPort,
    OptimizeSessionMutationApplicationPort,
};
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::{GenerationTaskAccessError, GenerationTaskStart};

use super::{
    OptimizationPatch, OptimizeAgentPermissions, OptimizeChatGenerationApplicationContext,
    OptimizePatchGenerationApplicationContext, apply_optimization_patches_to_content,
    optimization_agent_variables, optimization_analysis_variables, optimization_content_hash,
};

const PATCH_SET_STATUS_PENDING: &str = "pending";
const PATCH_SET_STATUS_APPLIED: &str = "applied";
const PATCH_SET_STATUS_REJECTED: &str = "rejected";
const PATCH_DECISION_ACCEPTED: &str = "accepted";
const PATCH_DECISION_REJECTED: &str = "rejected";
const PATCH_APPLY_STATUS_PENDING: &str = "pending";
const PATCH_APPLY_STATUS_APPLIED: &str = "applied";

pub struct PrepareOptimizeAnalysisApplicationInput<'a> {
    pub user_id: i64,
    pub session_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, serde_json::Value>,
}

pub struct ApplyOptimizePatchesApplicationInput<'a> {
    pub user_id: i64,
    pub session_id: i64,
    pub base_content_hash: &'a str,
    pub patches: &'a [OptimizationPatch],
    pub host_port: &'a dyn OptimizePatchApplyHostPort,
}

pub struct ApplyOptimizePatchesApplicationResult {
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
}

impl OrchestratorContext {
    async fn optimize_chapter_for_user(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<a4n_db_models::models::chapter::chapter::Model, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.chapter_by_project_number(project_id, chapter_number)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "章节不存在: project_id={project_id}, chapter_number={chapter_number}"
                ))
            })
    }

    pub async fn create_optimize_analysis_session_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        name: &str,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<optimization_session::Model, AppError> {
        let chapter = self
            .optimize_chapter_for_user(user_id, project_id, chapter_number)
            .await?;
        self.create_analysis_session(
            project_id,
            chapter.id,
            name,
            &chapter.content,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn create_optimize_chat_session_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        name: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<optimization_session::Model, AppError> {
        let chapter = self
            .optimize_chapter_for_user(user_id, project_id, chapter_number)
            .await?;
        self.start_chat_session(project_id, chapter.id, name, variables)
            .await
    }

    pub(crate) async fn validated_optimize_chat_generation_session(
        &self,
        user_id: i64,
        session_id: i64,
        message: &str,
    ) -> Result<optimization_session::Model, AppError> {
        crate::optimize::chat::validation::required_chat_user_text(message)?;
        let session = self.optimize_session_for_user(user_id, session_id).await?;
        crate::optimize::chat::validation::ensure_chat_session_accepts_message(
            &session,
            "发送内容优化 Agent 对话",
        )?;
        self.load_optimize_chapter_for_operation(
            &session,
            "发送内容优化 Agent 对话",
            "已删除章节不能继续内容优化对话",
            None,
        )
        .await?;
        Ok(session)
    }

    pub async fn build_create_optimize_analysis_preview_request_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let chapter = self
            .optimize_chapter_for_user(user_id, project_id, chapter_number)
            .await?;
        self.build_analysis_preview_request(
            project_id,
            chapter.id,
            &chapter.content,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn build_prepare_optimize_analysis_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let session = self.optimize_session_for_user(user_id, session_id).await?;
        let chapter = self
            .optimization_chapter_by_id(session.chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
        self.build_analysis_preview_request(
            session.project_id,
            session.chapter_id,
            &chapter.content,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn build_send_user_message_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        message: &str,
        permissions: &OptimizeAgentPermissions,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        self.build_send_user_message_preview_request(
            session_id,
            message,
            permissions,
            prompt_renderer,
        )
        .await
    }

    pub async fn build_propose_patches_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        issue_ids: Option<Vec<String>>,
        prompt_renderer: &dyn PromptRenderer,
        agent_instruction_override: Option<&str>,
    ) -> Result<ChatRequest, AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        self.build_propose_patches_preview_request(
            session_id,
            issue_ids,
            prompt_renderer,
            agent_instruction_override,
        )
        .await
    }

    pub async fn prepare_optimize_analysis_session_validated(
        &self,
        input: PrepareOptimizeAnalysisApplicationInput<'_>,
    ) -> Result<optimization_session::Model, AppError> {
        let session = self
            .optimize_session_for_user(input.user_id, input.session_id)
            .await?;
        let result = async {
            let chapter = self
                .optimization_chapter_by_id(session.chapter_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
            self.prepare_analysis_session(
                input.session_id,
                &chapter.content,
                input.prompt_renderer,
                input.variables,
            )
            .await
        }
        .await;
        self.record_optimize_operation_failure(input.session_id, "准备内容优化分析", result)
            .await
    }

    pub async fn apply_optimize_patches_validated(
        &self,
        input: ApplyOptimizePatchesApplicationInput<'_>,
    ) -> Result<ApplyOptimizePatchesApplicationResult, AppError> {
        let session = self
            .optimize_session_for_user(input.user_id, input.session_id)
            .await?;
        let result = async {
            if input.patches.is_empty() {
                return Err(AppError::InvalidArgument(
                    "应用补丁列表不能为空".to_string(),
                ));
            }
            let chapter = self
                .optimization_chapter_by_id(session.chapter_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
            if chapter.status == "deleted" {
                return Err(AppError::InvalidArgument(
                    "已删除章节不能应用局部补丁".to_string(),
                ));
            }
            if chapter.status == "generating" {
                return Err(AppError::InvalidState {
                    current: chapter.status,
                    operation: "应用局部补丁".to_string(),
                });
            }
            let next_content = apply_optimization_patches_to_content(
                &chapter.content,
                input.base_content_hash,
                input.patches,
            )?;
            let word_count = i32::try_from(next_content.chars().count()).map_err(|_| {
                AppError::InvalidArgument("应用补丁后的章节正文字符数超出 i32 范围".to_string())
            })?;
            let written = self
                .write_optimization_chapter_patch_content(OptimizationChapterPatchWriteInput {
                    chapter,
                    content: next_content,
                    word_count,
                })
                .await?;
            input
                .host_port
                .postprocess_chapter_after_content_change(written.id)
                .await?;
            let updated = self
                .optimization_chapter_by_id(written.id)
                .await?
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "应用局部补丁后刷新章节失败: 章节不存在: {}",
                        written.id
                    ))
                })?;
            let content_hash = optimization_content_hash(&updated.content);
            self.record_patches_applied(
                input.session_id,
                updated.id,
                updated.word_count,
                &content_hash,
                input.patches,
            )
            .await?;
            self.mark_matching_patch_sets_applied(input.session_id, input.base_content_hash)
                .await?;
            Ok(ApplyOptimizePatchesApplicationResult {
                chapter_id: updated.id,
                applied_count: input.patches.len(),
                word_count: updated.word_count,
            })
        }
        .await;
        self.record_optimize_operation_failure(input.session_id, "应用局部补丁", result)
            .await
    }

    pub async fn discard_pending_optimize_patch_sets_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<(), AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        self.discard_pending_optimization_patch_sets(
            session_id,
            PATCH_SET_STATUS_PENDING,
            PATCH_SET_STATUS_REJECTED,
            PATCH_DECISION_REJECTED,
            PATCH_APPLY_STATUS_PENDING,
            "用户已丢弃当前待应用局部补丁。",
        )
        .await
    }

    async fn mark_matching_patch_sets_applied(
        &self,
        session_id: i64,
        base_content_hash: &str,
    ) -> Result<(), AppError> {
        let patch_sets = self
            .matching_pending_optimization_patch_sets(
                session_id,
                base_content_hash,
                PATCH_SET_STATUS_PENDING,
            )
            .await?;
        let patch_set_ids = patch_sets
            .iter()
            .map(|patch_set| patch_set.id)
            .collect::<Vec<_>>();
        self.update_optimization_patch_sets_status(
            &patch_set_ids,
            PATCH_SET_STATUS_APPLIED,
            PATCH_DECISION_ACCEPTED,
            PATCH_APPLY_STATUS_APPLIED,
        )
        .await?;
        Ok(())
    }

    async fn record_optimize_operation_failure<T>(
        &self,
        session_id: i64,
        phase: &str,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => match self
                .record_command_error(
                    session_id,
                    phase,
                    &error.to_string(),
                    matches!(error, AppError::Cancelled),
                )
                .await
            {
                Ok(()) => Err(error),
                Err(record_error) => Err(AppError::InvalidState {
                    current: error.to_string(),
                    operation: format!("{phase}失败，且优化失败事件写入失败: {record_error}"),
                }),
            },
        }
    }
}

#[async_trait]
impl a4n_api_contract::optimize::OptimizeSessionReadApplicationPort for OrchestratorContext {
    async fn list_sessions(
        &self,
        user_id: i64,
        input: ListOptimizeSessionsInput,
    ) -> Result<Vec<OptimizationSessionSummary>, AppError> {
        OrchestratorContext::optimize_sessions_for_user(
            self,
            user_id,
            input.project_id,
            input.chapter_id,
        )
        .await?
        .iter()
        .map(optimization_session_summary)
        .collect()
    }

    async fn get_session(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<OptimizationSessionSummary, AppError> {
        optimization_session_summary(
            &OrchestratorContext::optimize_session_for_user(self, user_id, input.session_id)
                .await?,
        )
    }

    async fn list_turns(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<Vec<OptimizationTurnSummary>, AppError> {
        OrchestratorContext::optimize_turns_for_user(self, user_id, input.session_id)
            .await?
            .into_iter()
            .map(|(turn, events)| optimization_turn_summary(&turn, events))
            .collect()
    }

    async fn list_patch_sets(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<Vec<OptimizationPatchSetSummary>, AppError> {
        OrchestratorContext::optimize_patch_sets_for_user(self, user_id, input.session_id)
            .await?
            .into_iter()
            .map(|(patch_set, patches)| optimization_patch_set_summary(&patch_set, patches))
            .collect()
    }
}

#[async_trait]
impl OptimizeSessionMutationApplicationPort for OrchestratorContext {
    async fn create_analysis_session(
        &self,
        user_id: i64,
        input: AnalyzeChapterInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<OptimizationSessionSummary, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        let session = OrchestratorContext::create_optimize_analysis_session_for_user_validated(
            self,
            user_id,
            input.project_id,
            input.chapter_number,
            &input.name,
            prompt_renderer,
            &variables,
        )
        .await?;
        optimization_session_summary(&session)
    }

    async fn prepare_analysis_session(
        &self,
        user_id: i64,
        input: PrepareOptimizationAnalysisSessionInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<OptimizationSessionSummary, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        let session = OrchestratorContext::prepare_optimize_analysis_session_validated(
            self,
            PrepareOptimizeAnalysisApplicationInput {
                user_id,
                session_id: input.session_id,
                prompt_renderer,
                variables: &variables,
            },
        )
        .await?;
        optimization_session_summary(&session)
    }

    async fn create_chat_session(
        &self,
        user_id: i64,
        input: CreateOptimizationChatSessionInput,
    ) -> Result<OptimizationSessionSummary, AppError> {
        let variables = optimization_agent_variables(&input.permissions);
        let session = OrchestratorContext::create_optimize_chat_session_for_user_validated(
            self,
            user_id,
            input.project_id,
            input.chapter_number,
            &input.name,
            &variables,
        )
        .await?;
        optimization_session_summary(&session)
    }

    async fn delete_session(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<(), AppError> {
        OrchestratorContext::delete_optimize_session_for_user_validated(
            self,
            user_id,
            input.session_id,
        )
        .await
    }

    async fn record_agent_event(
        &self,
        user_id: i64,
        input: RecordOptimizeAgentEventInput,
    ) -> Result<(), AppError> {
        OrchestratorContext::record_optimize_agent_event_for_user_validated(
            self,
            user_id,
            input.session_id,
            &input.content,
        )
        .await
    }

    async fn discard_pending_patch_sets(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<(), AppError> {
        OrchestratorContext::discard_pending_optimize_patch_sets_for_user_validated(
            self,
            user_id,
            input.session_id,
        )
        .await
    }
}

#[async_trait]
impl OptimizePatchApplicationPort for OrchestratorContext {
    async fn apply_patches(
        &self,
        user_id: i64,
        input: ApplyOptimizationPatchesInput,
        host_port: &dyn OptimizePatchApplyHostPort,
    ) -> Result<ApplyOptimizationPatchesResponse, AppError> {
        let result = OrchestratorContext::apply_optimize_patches_validated(
            self,
            ApplyOptimizePatchesApplicationInput {
                user_id,
                session_id: input.session_id,
                base_content_hash: &input.base_content_hash,
                patches: &input.patches,
                host_port,
            },
        )
        .await?;
        Ok(ApplyOptimizationPatchesResponse {
            chapter_id: result.chapter_id,
            applied_count: result.applied_count,
            word_count: result.word_count,
        })
    }
}

#[async_trait]
impl OptimizeAnalysisPreviewApplicationPort for OrchestratorContext {
    async fn preview_create_analysis(
        &self,
        user_id: i64,
        input: AnalyzeChapterInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        OrchestratorContext::build_create_optimize_analysis_preview_request_for_user_validated(
            self,
            user_id,
            input.project_id,
            input.chapter_number,
            prompt_renderer,
            &variables,
        )
        .await
    }

    async fn preview_prepare_analysis(
        &self,
        user_id: i64,
        input: PrepareOptimizationAnalysisSessionInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        OrchestratorContext::build_prepare_optimize_analysis_preview_request_for_user_validated(
            self,
            user_id,
            input.session_id,
            prompt_renderer,
            &variables,
        )
        .await
    }
}

#[async_trait]
impl OptimizeChatPreviewApplicationPort for OrchestratorContext {
    async fn preview_send_message(
        &self,
        user_id: i64,
        input: SendOptimizationMessageInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        OrchestratorContext::build_send_user_message_preview_request_for_user_validated(
            self,
            user_id,
            input.session_id,
            &input.message,
            &input.permissions,
            prompt_renderer,
        )
        .await
    }
}

#[async_trait]
impl OptimizePatchPreviewApplicationPort for OrchestratorContext {
    async fn preview_propose_patches(
        &self,
        user_id: i64,
        input: a4n_api_contract::optimize::ProposeOptimizationPatchesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        OrchestratorContext::build_propose_patches_preview_request_for_user_validated(
            self,
            user_id,
            input.session_id,
            input.issue_ids,
            prompt_renderer,
            input.agent_instruction.as_deref(),
        )
        .await
    }
}

#[async_trait]
impl OptimizeChatGenerationApplicationPort for OrchestratorContext {
    async fn start_chat_generation(
        &self,
        user_id: i64,
        input: &StartOptimizeChatGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_optimize_chat_generation_task(self, user_id, input)
            .await
            .map_err(GenerationTaskAccessError::into_start_error)
    }

    async fn run_chat_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        agent_max_output_tokens: Option<usize>,
        user_id: i64,
        input: StartOptimizeChatGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_optimize_chat_generation_task(
            self,
            OptimizeChatGenerationApplicationContext {
                prompt_renderer,
                agent_max_output_tokens,
            },
            user_id,
            input,
            handle,
        )
        .await
    }
}

#[async_trait]
impl OptimizeAnalysisGenerationApplicationPort for OrchestratorContext {
    async fn start_analysis_generation(
        &self,
        user_id: i64,
        input: &StartOptimizeAnalysisGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_optimize_analysis_generation_task(self, user_id, input)
            .await
            .map_err(GenerationTaskAccessError::into_start_error)
    }

    async fn run_analysis_generation(
        &self,
        user_id: i64,
        input: StartOptimizeAnalysisGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_optimize_analysis_generation_task(self, user_id, input, handle)
            .await
    }
}

#[async_trait]
impl OptimizePatchGenerationApplicationPort for OrchestratorContext {
    async fn start_patch_generation(
        &self,
        user_id: i64,
        input: &StartOptimizePatchGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_optimize_patch_generation_task(self, user_id, input)
            .await
            .map_err(GenerationTaskAccessError::into_start_error)
    }

    async fn run_patch_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        user_id: i64,
        input: StartOptimizePatchGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_optimize_patch_generation_task(
            self,
            OptimizePatchGenerationApplicationContext { prompt_renderer },
            user_id,
            input,
            handle,
        )
        .await
    }
}

fn optimization_session_summary(
    model: &optimization_session::Model,
) -> Result<OptimizationSessionSummary, AppError> {
    Ok(OptimizationSessionSummary {
        id: model.id,
        project_id: model.project_id,
        chapter_id: model.chapter_id,
        name: model.name.clone(),
        status: model.status.clone(),
        issues: model.issues.clone(),
        agent_permissions: required_permissions(
            model.agent_permissions.as_ref(),
            "optimization_session.agent_permissions",
        )?,
        agent_instruction: model.agent_instruction.clone(),
        fixes_applied: model.fixes_applied,
        summary: model.summary.clone(),
        created_at: format_utc_timestamp(model.created_at),
        updated_at: format_utc_timestamp(model.updated_at),
    })
}

fn optimization_turn_summary(
    turn: &optimization_turn::Model,
    events: Vec<optimization_turn_event::Model>,
) -> Result<OptimizationTurnSummary, AppError> {
    Ok(OptimizationTurnSummary {
        id: turn.id,
        session_id: turn.session_id,
        user_text: turn.user_text.clone(),
        status: turn.status.clone(),
        permissions_snapshot: required_permissions(
            turn.permissions_snapshot.as_ref(),
            "optimization_turn.permissions_snapshot",
        )?,
        final_reply: turn.final_reply.clone(),
        error: turn.error.clone(),
        events: events
            .into_iter()
            .map(
                |event| a4n_api_contract::optimize::OptimizationTurnEventSummary {
                    id: event.id,
                    turn_id: event.turn_id,
                    seq: event.seq,
                    kind: event.kind,
                    status: event.status,
                    public_text: event.public_text,
                    payload: event.payload,
                    created_at: format_utc_timestamp(event.created_at),
                },
            )
            .collect(),
        created_at: format_utc_timestamp(turn.created_at),
        updated_at: format_utc_timestamp(turn.updated_at),
    })
}

fn optimization_patch_set_summary(
    patch_set: &optimization_patch_set::Model,
    patches: Vec<optimization_patch::Model>,
) -> Result<OptimizationPatchSetSummary, AppError> {
    Ok(OptimizationPatchSetSummary {
        id: patch_set.id,
        turn_id: patch_set.turn_id,
        session_id: patch_set.session_id,
        chapter_id: patch_set.chapter_id,
        base_content_hash: patch_set.base_content_hash.clone(),
        status: patch_set.status.clone(),
        summary: patch_set.summary.clone(),
        patches: patches
            .into_iter()
            .map(|patch| {
                Ok(OptimizationPatch {
                    patch_id: patch.patch_id,
                    issue_id: patch.issue_id,
                    operation: parse_patch_operation(&patch.operation)?,
                    old_text: patch.old_text,
                    new_text: patch.new_text,
                    reason: patch.reason,
                    risk: patch.risk,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
        created_at: format_utc_timestamp(patch_set.created_at),
        updated_at: format_utc_timestamp(patch_set.updated_at),
    })
}

fn format_utc_timestamp(value: chrono::NaiveDateTime) -> String {
    value.and_utc().to_rfc3339()
}

fn required_permissions(
    value: Option<&serde_json::Value>,
    field: &str,
) -> Result<OptimizeAgentPermissions, AppError> {
    serde_json::from_value(value.cloned().ok_or_else(|| AppError::InvalidState {
        current: format!("{field}_missing"),
        operation: format!("读取 {field}"),
    })?)
    .map_err(|error| AppError::InvalidState {
        current: error.to_string(),
        operation: format!("读取 {field}"),
    })
}

fn parse_patch_operation(
    value: &str,
) -> Result<a4n_api_contract::optimize::OptimizationPatchOperation, AppError> {
    match value {
        "replace" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::Replace),
        "delete" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::Delete),
        "insert_before" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::InsertBefore),
        "insert_after" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::InsertAfter),
        _ => Err(AppError::InvalidState {
            current: value.to_string(),
            operation: "读取 optimization_patch.operation".to_string(),
        }),
    }
}

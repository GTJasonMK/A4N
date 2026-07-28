use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;

use super::{
    OptimizationChapterPatchWriteInput, OptimizationPatch, OptimizeAgentPermissions,
    apply_optimization_patches_to_content, optimization_content_hash,
};

const PATCH_SET_STATUS_PENDING: &str = "pending";
const PATCH_SET_STATUS_APPLIED: &str = "applied";
const PATCH_SET_STATUS_REJECTED: &str = "rejected";
const PATCH_DECISION_ACCEPTED: &str = "accepted";
const PATCH_DECISION_REJECTED: &str = "rejected";
const PATCH_APPLY_STATUS_PENDING: &str = "pending";
const PATCH_APPLY_STATUS_APPLIED: &str = "applied";

#[async_trait]
pub trait OptimizePatchApplyHostPort: Send + Sync {
    async fn postprocess_chapter_after_content_change(
        &self,
        chapter_id: i64,
    ) -> Result<(), AppError>;
}

pub struct PrepareOptimizeAnalysisApplicationInput<'a> {
    pub user_id: i64,
    pub session_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, serde_json::Value>,
}

pub struct ApplyOptimizePatchesApplicationInput<'a> {
    pub user_id: i64,
    pub session_id: i64,
    pub confirmed_apply: bool,
    pub base_content_hash: &'a str,
    pub patches: &'a [OptimizationPatch],
    pub host_port: &'a dyn OptimizePatchApplyHostPort,
}

pub struct ApplyOptimizePatchesApplicationResult {
    pub session_id: i64,
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
    pub content_hash: String,
}

impl OrchestratorContext {
    async fn optimize_chapter_for_user(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<a4n_core::models::chapter::chapter::Model, AppError> {
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
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
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
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
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
        permissions_override: Option<&OptimizeAgentPermissions>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        self.build_send_user_message_preview_request(
            session_id,
            message,
            permissions_override,
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
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
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
            if !input.confirmed_apply {
                return Err(AppError::InvalidArgument(
                    "应用局部补丁必须经过用户确认".to_string(),
                ));
            }
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
                session_id: input.session_id,
                chapter_id: updated.id,
                applied_count: input.patches.len(),
                word_count: updated.word_count,
                content_hash,
            })
        }
        .await;
        self.record_optimize_operation_failure(input.session_id, "应用局部补丁", result)
            .await
    }

    pub async fn reject_pending_optimize_patch_sets_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<(), AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        let patch_sets = self
            .pending_optimization_patch_sets(session_id, PATCH_SET_STATUS_PENDING)
            .await?;
        for patch_set in patch_sets {
            self.update_optimization_patch_set_status(
                patch_set,
                PATCH_SET_STATUS_REJECTED,
                PATCH_DECISION_REJECTED,
                PATCH_APPLY_STATUS_PENDING,
            )
            .await?;
        }
        Ok(())
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
        for patch_set in patch_sets {
            self.update_optimization_patch_set_status(
                patch_set,
                PATCH_SET_STATUS_APPLIED,
                PATCH_DECISION_ACCEPTED,
                PATCH_APPLY_STATUS_APPLIED,
            )
            .await?;
        }
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

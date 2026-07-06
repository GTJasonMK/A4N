use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_engine::llm::provider::{ChatRequest, LlmProvider};
use a4n_engine::prompt::engine::PromptEngine;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;

use super::agent_loop::{build_first_agent_loop_chat_request, insert_current_chapter_content};
use super::agent_loop_executor::ExplicitAnalysisAgentLoopInput;
use super::context::{build_content_optimization_context, optimization_context_manifest};
use super::messages::{
    build_content_optimization_user_message, is_content_optimization_analysis_prompt,
    optimization_dimensions,
};
use super::permissions::{
    OptimizeAgentPermissions, insert_optimization_agent_capability_variables,
};
use super::{
    OptimizeLlmStream, OptimizeProgress, OptimizeService, report_optimize_progress,
    required_available_session_name,
};

struct AnalysisSessionPromptBundle {
    analysis_prompt: String,
    context_manifest: String,
    user_message: String,
    permissions: OptimizeAgentPermissions,
    agent_instruction: Option<String>,
}

impl OptimizeService {
    /// 创建优化分析会话，但不立即调用模型。
    pub async fn create_analysis_session(
        &self,
        project_id: i64,
        chapter_id: i64,
        name: &str,
        chapter_content: &str,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<optimization_session::Model, AppError> {
        let name =
            required_available_session_name(self.db.as_ref(), project_id, chapter_id, name).await?;
        let bundle = self
            .build_analysis_session_prompt_bundle(
                project_id,
                chapter_id,
                chapter_content,
                prompt_engine,
                variables,
            )
            .await?;
        let now = Utc::now().naive_utc();
        let session = optimization_session::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter_id),
            name: Set(Some(name)),
            status: Set("analyzing".to_string()),
            issues: Set(None),
            agent_permissions: Set(Some(bundle.permissions.as_value())),
            agent_instruction: Set(bundle.agent_instruction.clone()),
            fixes_applied: Set(0),
            summary: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;

        for (role, content) in [
            ("system", bundle.analysis_prompt.as_str()),
            ("context", bundle.context_manifest.as_str()),
        ] {
            if let Err(err) = self.save_message(session.id, role, content).await {
                self.cancel_session(session.id).await?;
                return Err(err);
            }
        }
        if let Some(instruction) = bundle.agent_instruction.as_deref()
            && let Err(err) = self.save_message(session.id, "user", instruction).await
        {
            self.cancel_session(session.id).await?;
            return Err(err);
        }
        if let Err(err) = self
            .save_message(session.id, "user", &bundle.user_message)
            .await
        {
            self.cancel_session(session.id).await?;
            return Err(err);
        }
        if let Err(err) = self
            .save_agent_event(session.id, "分析会话已创建，等待执行模型分析。")
            .await
        {
            self.cancel_session(session.id).await?;
            return Err(err);
        }

        Ok(session)
    }

    /// 在已有 Agent 会话内准备一次新的章节分析输入，不新建会话。
    pub async fn prepare_analysis_session(
        &self,
        session_id: i64,
        chapter_content: &str,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<optimization_session::Model, AppError> {
        let session = self.get_session(session_id).await?;
        if matches!(
            session.status.as_str(),
            "analyzing" | "fixing" | "cancelled"
        ) {
            return Err(AppError::InvalidState {
                current: session.status,
                operation: "准备内容优化分析".to_string(),
            });
        }

        let permissions = if variables.contains_key("agent_permissions") {
            OptimizeAgentPermissions::from_variables(variables)
        } else {
            OptimizeAgentPermissions::from_session(&session)
        };
        let mut effective_variables = variables.clone();
        effective_variables.insert("agent_permissions".to_string(), permissions.as_value());
        let bundle = self
            .build_analysis_session_prompt_bundle(
                session.project_id,
                session.chapter_id,
                chapter_content,
                prompt_engine,
                &effective_variables,
            )
            .await?;

        self.save_message(session.id, "system", &bundle.analysis_prompt)
            .await?;
        self.save_message(session.id, "context", &bundle.context_manifest)
            .await?;
        if let Some(instruction) = bundle.agent_instruction.as_deref() {
            self.save_message(session.id, "user", instruction).await?;
        }
        self.save_message(session.id, "user", &bundle.user_message)
            .await?;
        self.save_agent_event(session.id, "已在当前会话准备分析输入，等待执行模型分析。")
            .await?;

        let mut active: optimization_session::ActiveModel = session.into();
        active.status = Set("analyzing".to_string());
        active.agent_permissions = Set(Some(bundle.permissions.as_value()));
        active.agent_instruction = Set(bundle.agent_instruction.clone());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 执行已经创建的优化分析会话。
    pub async fn run_analysis_session(
        &self,
        session_id: i64,
        llm: &dyn LlmProvider,
        workflow_run_id: i64,
        mut progress: Option<OptimizeProgress<'_>>,
        mut stream: Option<OptimizeLlmStream<'_>>,
    ) -> Result<optimization_session::Model, AppError> {
        report_optimize_progress(&mut progress, 0, 5, "正在读取分析会话上下文")?;
        let session = self.get_session(session_id).await?;
        if session.status != "analyzing" {
            return Err(AppError::InvalidState {
                current: session.status,
                operation: "执行内容优化分析".to_string(),
            });
        }

        let session_messages = self.session_messages(session_id).await?;
        let analysis_prompt = session_messages
            .iter()
            .rev()
            .find(|message| message.role == "system")
            .map(|message| message.content.clone())
            .ok_or_else(|| AppError::InvalidArgument("优化分析会话缺少 system 提示词".into()))?;
        let user_message = session_messages
            .iter()
            .rev()
            .find(|message| {
                message.role == "user" && is_content_optimization_analysis_prompt(&message.content)
            })
            .map(|message| message.content.clone())
            .ok_or_else(|| AppError::InvalidArgument("优化分析会话缺少分析 user 输入".into()))?;

        let chapter = match self
            .load_optimize_chapter_for_operation(
                &session,
                "执行内容优化分析",
                "已删除章节不能执行内容优化分析",
                Some("章节正文为空，无法执行内容优化分析"),
            )
            .await
        {
            Ok(chapter) => chapter,
            Err(err) => {
                self.release_in_progress_session_after_failure(
                    session.id,
                    "内容优化 Agent Loop",
                    &err.to_string(),
                )
                .await?;
                return Err(err);
            }
        };

        let permissions = OptimizeAgentPermissions::from_session(&session);
        let mut analysis_context = match build_content_optimization_context(
            self.db.as_ref(),
            session.project_id,
            session.chapter_id,
            &permissions,
        )
        .await
        {
            Ok(context) => context,
            Err(err) => {
                self.save_agent_event(session.id, &format!("读取 Agent Loop 上下文失败：{err}"))
                    .await?;
                self.release_in_progress_session_after_failure(
                    session.id,
                    "内容优化 Agent Loop",
                    &err.to_string(),
                )
                .await?;
                return Err(err);
            }
        };
        insert_current_chapter_content(&mut analysis_context, &chapter.content);
        let turn_text = session
            .agent_instruction
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .unwrap_or("分析当前章节");
        let turn = self
            .create_turn(session.id, turn_text, &permissions)
            .await?;

        match self
            .run_explicit_analysis_agent_loop(ExplicitAnalysisAgentLoopInput {
                turn_id: turn.id,
                session,
                analysis_prompt,
                base_user_message: user_message,
                analysis_context,
                llm,
                workflow_run_id,
                progress: &mut progress,
                stream: &mut stream,
            })
            .await
        {
            Ok(session) => {
                self.finish_turn(turn.id, "分析完成，问题列表已保存到当前会话。")
                    .await?;
                Ok(session)
            }
            Err(err) => {
                let status = if super::is_cancelled_optimize_error(&err.to_string()) {
                    super::turns::TURN_STATUS_CANCELLED
                } else {
                    super::turns::TURN_STATUS_FAILED
                };
                self.fail_turn(turn.id, status, &err.to_string()).await?;
                Err(err)
            }
        }
    }

    pub async fn build_analysis_preview_request(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_content: &str,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let bundle = self
            .build_analysis_session_prompt_bundle(
                project_id,
                chapter_id,
                chapter_content,
                prompt_engine,
                variables,
            )
            .await?;
        build_first_agent_loop_chat_request(
            project_id,
            None,
            "content_optimization_agent_loop",
            bundle.analysis_prompt,
            &bundle.user_message,
        )
    }

    async fn build_analysis_session_prompt_bundle(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_content: &str,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<AnalysisSessionPromptBundle, AppError> {
        let permissions = OptimizeAgentPermissions::from_variables(variables);
        let agent_instruction = variables
            .get("agent_instruction")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned);
        let prompt_vars = self.build_analysis_prompt_variables(variables)?;
        let analysis_prompt =
            prompt_engine.render("content_optimization", &prompt_vars, Some(project_id))?;
        let analysis_context = build_content_optimization_context(
            self.db.as_ref(),
            project_id,
            chapter_id,
            &permissions,
        )
        .await?;
        let context_manifest_value = optimization_context_manifest(&analysis_context);
        let context_manifest = serde_json::to_string_pretty(&context_manifest_value)?;
        let user_message = build_content_optimization_user_message(
            chapter_content,
            Some(&context_manifest_value),
            &permissions,
            agent_instruction.as_deref(),
        )?;

        Ok(AnalysisSessionPromptBundle {
            analysis_prompt,
            context_manifest,
            user_message,
            permissions,
            agent_instruction,
        })
    }

    fn build_analysis_prompt_variables(
        &self,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>, AppError> {
        let mut prompt_vars = variables.clone();
        let mode = variables
            .get("mode")
            .and_then(|value| value.as_str())
            .ok_or_else(|| AppError::InvalidArgument("缺少优化分析模式 mode".into()))?;
        prompt_vars.insert(
            "dimensions".to_string(),
            serde_json::Value::String(optimization_dimensions(mode)?.to_string()),
        );
        prompt_vars.insert(
            "tools_prompt".to_string(),
            serde_json::Value::String(serde_json::to_string_pretty(&self.tool_specs())?),
        );
        let permissions = OptimizeAgentPermissions::from_variables(variables);
        prompt_vars.insert("agent_permissions".to_string(), permissions.as_value());
        prompt_vars.insert(
            "agent_permissions_prompt".to_string(),
            serde_json::Value::String(serde_json::to_string_pretty(&permissions.as_value())?),
        );
        insert_optimization_agent_capability_variables(&mut prompt_vars, &permissions)?;
        prompt_vars.insert(
            "agent_instruction".to_string(),
            variables
                .get("agent_instruction")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .unwrap_or("无额外用户指令")
                .to_string()
                .into(),
        );
        Ok(prompt_vars)
    }
}

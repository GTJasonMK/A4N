use super::super::agent_loop::insert_current_chapter_content;
use super::super::agent_loop_executor::{
    AgentLoopExecutionError, AgentLoopGenerationContext, ExplicitAnalysisAgentLoopInput,
    OptimizeAnalysisTurnExecution,
};
use super::super::context::build_content_optimization_context;
use super::super::messages::is_content_optimization_analysis_prompt;
use super::super::{OptimizeAgentPermissions, OrchestratorContext};

impl OrchestratorContext {
    /// 执行已经创建的优化分析会话，结果由 generation terminal 事务正式提交。
    pub(in crate::optimize) async fn run_analysis_session(
        &self,
        session_id: i64,
        workflow_run_id: i64,
        generation: &AgentLoopGenerationContext<'_>,
    ) -> Result<OptimizeAnalysisTurnExecution, AgentLoopExecutionError> {
        let session = self.get_session(session_id).await?;
        if session.status != "analyzing" {
            return Err(a4n_core::error::AppError::InvalidState {
                current: session.status,
                operation: "执行内容优化分析".to_string(),
            }
            .into());
        }

        let session_messages = self.session_messages(session_id).await?;
        let analysis_prompt = session_messages
            .iter()
            .rev()
            .find(|message| message.role == "system")
            .map(|message| message.content.clone())
            .ok_or_else(|| {
                a4n_core::error::AppError::InvalidArgument("优化分析会话缺少 system 提示词".into())
            })?;
        let user_message = session_messages
            .iter()
            .rev()
            .find(|message| {
                message.role == "user" && is_content_optimization_analysis_prompt(&message.content)
            })
            .map(|message| message.content.clone())
            .ok_or_else(|| {
                a4n_core::error::AppError::InvalidArgument("优化分析会话缺少分析 user 输入".into())
            })?;

        let chapter = self
            .load_optimize_chapter_for_operation(
                &session,
                "执行内容优化分析",
                "已删除章节不能执行内容优化分析",
                Some("章节正文为空，无法执行内容优化分析"),
            )
            .await?;
        let permissions = OptimizeAgentPermissions::from_session(&session);
        let mut analysis_context = build_content_optimization_context(
            self.database.connection(),
            session.project_id,
            session.chapter_id,
            &permissions,
        )
        .await?;
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
                workflow_run_id,
                generation,
            })
            .await
        {
            Ok(result) => Ok(result),
            Err(err) => {
                let status = if err.is_cancelled() {
                    crate::optimize::turns::TURN_STATUS_CANCELLED
                } else {
                    crate::optimize::turns::TURN_STATUS_FAILED
                };
                self.fail_turn(turn.id, status, &err.to_string()).await?;
                Err(err)
            }
        }
    }
}

use super::*;

impl OrchestratorContext {
    /// 执行已经创建的优化分析会话。
    pub async fn run_analysis_session(
        &self,
        session_id: i64,
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
                    &err,
                )
                .await?;
                return Err(err);
            }
        };

        let permissions = OptimizeAgentPermissions::from_session(&session);
        let mut analysis_context = match build_content_optimization_context(
            self.db.connection(),
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
                    &err,
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
                let status = if crate::optimize::is_cancelled_optimize_error(&err) {
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

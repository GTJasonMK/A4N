use super::*;

impl OrchestratorContext {
    pub(crate) async fn create_analysis_session(
        &self,
        project_id: i64,
        chapter_id: i64,
        name: &str,
        chapter_content: &str,
        prompt_renderer: &dyn PromptRenderer,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<optimization_session::Model, AppError> {
        let name = required_available_session_name(
            self.database.connection(),
            project_id,
            chapter_id,
            name,
        )
        .await?;
        let bundle = self
            .build_analysis_session_prompt_bundle(
                project_id,
                chapter_id,
                chapter_content,
                prompt_renderer,
                variables,
            )
            .await?;
        let session = a4n_db::optimize::create_optimization_session(
            self.database.connection(),
            OptimizationSessionCreateInput {
                project_id,
                chapter_id,
                name: Some(name),
                status: "analyzing".to_string(),
                issues: None,
                agent_permissions: Some(bundle.permissions.as_value()),
                agent_instruction: bundle.agent_instruction.clone(),
                fixes_applied: 0,
                summary: None,
            },
        )
        .await
        .map_err(optimize_db_error)?;

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
    pub(crate) async fn prepare_analysis_session(
        &self,
        session_id: i64,
        chapter_content: &str,
        prompt_renderer: &dyn PromptRenderer,
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

        let bundle = self
            .build_analysis_session_prompt_bundle(
                session.project_id,
                session.chapter_id,
                chapter_content,
                prompt_renderer,
                variables,
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

        a4n_db::optimize::update_optimization_session(
            self.database.connection(),
            OptimizationSessionUpdateInput {
                session,
                status: Some("analyzing".to_string()),
                issues: None,
                agent_permissions: Some(Some(bundle.permissions.as_value())),
                agent_instruction: Some(bundle.agent_instruction.clone()),
                fixes_applied: None,
                summary: None,
            },
        )
        .await
        .map_err(optimize_db_error)
    }
}

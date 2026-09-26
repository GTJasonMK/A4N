use super::*;

pub(in crate::optimize) struct AnalysisSessionPromptBundle {
    pub(in crate::optimize) analysis_prompt: String,
    pub(in crate::optimize) context_manifest: String,
    pub(in crate::optimize) user_message: String,
    pub(in crate::optimize) permissions: OptimizationAgentPermissions,
    pub(in crate::optimize) agent_instruction: Option<String>,
}

impl OrchestratorContext {
    pub(crate) async fn build_analysis_preview_request(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_content: &str,
        prompt_renderer: &dyn PromptRenderer,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let bundle = self
            .build_analysis_session_prompt_bundle(
                project_id,
                chapter_id,
                chapter_content,
                prompt_renderer,
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

    pub(in crate::optimize) async fn build_analysis_session_prompt_bundle(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_content: &str,
        prompt_renderer: &dyn PromptRenderer,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<AnalysisSessionPromptBundle, AppError> {
        let permissions = optimization_permissions_from_variables(variables)?;
        let agent_instruction = variables
            .get("agent_instruction")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned);
        let prompt_vars = self.build_analysis_prompt_variables(variables)?;
        let analysis_prompt = prompt_renderer
            .render_prompt("content_optimization", &prompt_vars, Some(project_id))
            .await?;
        let analysis_context = build_content_optimization_context(
            &self.database,
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
        let permissions = optimization_permissions_from_variables(variables)?;
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

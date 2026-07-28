use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;

use crate::optimize::context::build_content_optimization_context;
use crate::optimize::messages::{
    build_content_optimization_patch_user_message, select_optimization_issues,
};
use crate::optimize::patch::optimization_content_hash;
use crate::optimize::permissions::insert_optimization_agent_capability_variables;
use crate::optimize::{
    OrchestratorContext, PatchPromptBundle, PatchRequestBase,
    build_content_optimization_patch_request,
};
use a4n_prompt::rendering::PromptRenderer;

impl OrchestratorContext {
    pub(crate) async fn build_propose_patches_preview_request(
        &self,
        session_id: i64,
        issue_ids: Option<Vec<String>>,
        prompt_renderer: &dyn PromptRenderer,
        agent_instruction_override: Option<&str>,
    ) -> Result<ChatRequest, AppError> {
        let base = self
            .load_patch_request_base(session_id, agent_instruction_override, "预览局部修复补丁")
            .await?;
        Ok(self
            .build_patch_prompt_bundle(&base, issue_ids, prompt_renderer)
            .await?
            .request)
    }

    pub(in crate::optimize) async fn load_patch_request_base(
        &self,
        session_id: i64,
        agent_instruction_override: Option<&str>,
        operation: &str,
    ) -> Result<PatchRequestBase, AppError> {
        let session = self.get_session(session_id).await?;
        if matches!(
            session.status.as_str(),
            "analyzing" | "fixing" | "cancelled"
        ) {
            return Err(AppError::InvalidState {
                current: session.status,
                operation: operation.to_string(),
            });
        }
        let chapter = self
            .load_optimize_chapter_for_operation(
                &session,
                operation,
                "已删除章节不能生成局部补丁",
                Some("章节正文为空，无法生成局部补丁"),
            )
            .await?;
        let override_instruction = agent_instruction_override
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned);
        Ok(PatchRequestBase {
            session,
            chapter,
            override_instruction,
        })
    }

    pub(in crate::optimize) async fn build_patch_prompt_bundle(
        &self,
        base: &PatchRequestBase,
        issue_ids: Option<Vec<String>>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<PatchPromptBundle, AppError> {
        let selected_issues = select_optimization_issues(&base.session, issue_ids)?;
        let permissions = crate::optimize::OptimizeAgentPermissions::from_session(&base.session);
        let analysis_context = build_content_optimization_context(
            self.database.connection(),
            base.session.project_id,
            base.session.chapter_id,
            &permissions,
        )
        .await?;
        let base_content_hash = optimization_content_hash(&base.chapter.content);
        let mut prompt_vars = std::collections::HashMap::new();
        prompt_vars.insert("agent_permissions".to_string(), permissions.as_value());
        prompt_vars.insert(
            "agent_permissions_prompt".to_string(),
            serde_json::Value::String(serde_json::to_string_pretty(&permissions.as_value())?),
        );
        insert_optimization_agent_capability_variables(&mut prompt_vars, &permissions)?;
        prompt_vars.insert(
            "agent_instruction".to_string(),
            base.override_instruction
                .as_deref()
                .unwrap_or("无额外用户指令")
                .to_string()
                .into(),
        );
        let system_prompt = prompt_renderer
            .render_prompt(
                "content_optimization_patch",
                &prompt_vars,
                Some(base.session.project_id),
            )
            .await?;
        let user_message = build_content_optimization_patch_user_message(
            &base_content_hash,
            &selected_issues,
            &permissions,
            base.override_instruction.as_deref(),
        )?;
        let request = build_content_optimization_patch_request(
            base.session.project_id,
            base.session.id,
            system_prompt,
            user_message.clone(),
            &base.chapter.content,
            &analysis_context,
        )?;
        Ok(PatchPromptBundle {
            request,
            user_message,
            base_content_hash,
            issue_count: selected_issues.len(),
            permissions,
        })
    }
}

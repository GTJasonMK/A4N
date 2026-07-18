use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use serde_json::Value;

use crate::optimize::context::{build_content_optimization_context, optimization_context_manifest};
use crate::optimize::messages::{
    build_content_optimization_chat_user_message, optimization_chat_history,
};
use crate::optimize::{OptimizeAgentPermissions, OrchestratorContext};
use a4n_prompt::rendering::PromptRenderer;

pub(super) struct ChatAgentContext {
    pub(super) permissions: OptimizeAgentPermissions,
    pub(super) chat_context: Value,
    context_manifest: Value,
    conversation_history: Value,
}

impl OrchestratorContext {
    pub(super) async fn build_chat_agent_context(
        &self,
        session: &optimization_session::Model,
        permissions_override: Option<&OptimizeAgentPermissions>,
    ) -> Result<ChatAgentContext, AppError> {
        let previous_turns = self.list_turns(session.id).await?;
        let conversation_history = optimization_chat_history(&previous_turns);
        let permissions = permissions_override
            .cloned()
            .unwrap_or_else(|| OptimizeAgentPermissions::from_session(session));
        let analysis_context = build_content_optimization_context(
            self.db.connection(),
            session.project_id,
            session.chapter_id,
            &permissions,
        )
        .await?;
        let context_manifest = optimization_context_manifest(&analysis_context);
        Ok(ChatAgentContext {
            permissions,
            chat_context: analysis_context,
            context_manifest,
            conversation_history,
        })
    }

    pub(super) async fn build_chat_agent_prompt_bundle(
        &self,
        session: &optimization_session::Model,
        user_text: &str,
        chat_context: &ChatAgentContext,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<(String, String), AppError> {
        let mut prompt_vars = HashMap::new();
        prompt_vars.insert(
            "agent_instruction".to_string(),
            user_text.to_string().into(),
        );
        prompt_vars.insert(
            "tools_prompt".to_string(),
            serde_json::to_string_pretty(&self.tool_specs())?.into(),
        );
        let system_prompt = prompt_renderer
            .render_prompt(
                "content_optimization_chat",
                &prompt_vars,
                Some(session.project_id),
            )
            .await?;
        let base_user_message = build_content_optimization_chat_user_message(
            user_text,
            session,
            &chat_context.conversation_history,
            &chat_context.context_manifest,
        )?;
        Ok((system_prompt, base_user_message))
    }
}

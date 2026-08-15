use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use a4n_db::optimize::OptimizationSessionCreateInput;
use serde_json::Value;

use crate::optimize::context::{build_content_optimization_context, optimization_context_manifest};
use crate::optimize::permissions::optimization_permissions_from_variables;
use crate::optimize::{OrchestratorContext, optimize_db_error, required_available_session_name};

impl OrchestratorContext {
    /// 创建一个不先跑分析的普通交互会话。
    pub(crate) async fn start_chat_session(
        &self,
        project_id: i64,
        chapter_id: i64,
        name: &str,
        variables: &HashMap<String, Value>,
    ) -> Result<optimization_session::Model, AppError> {
        let name = required_available_session_name(
            self.database.connection(),
            project_id,
            chapter_id,
            name,
        )
        .await?;
        let permissions = optimization_permissions_from_variables(variables)?;
        let analysis_context = build_content_optimization_context(
            self.database.connection(),
            project_id,
            chapter_id,
            &permissions,
        )
        .await?;
        let context_manifest =
            serde_json::to_string_pretty(&optimization_context_manifest(&analysis_context))?;
        let session = a4n_db::optimize::create_optimization_session(
            self.database.connection(),
            OptimizationSessionCreateInput {
                project_id,
                chapter_id,
                name: Some(name),
                status: "waiting_user".to_string(),
                issues: None,
                agent_permissions: Some(permissions.as_value()),
                agent_instruction: None,
                fixes_applied: 0,
                summary: Some("普通交互会话".to_string()),
            },
        )
        .await
        .map_err(optimize_db_error)?;

        self.save_message(session.id, "context", &context_manifest)
            .await?;
        self.save_message(
            session.id,
            "agent_event",
            "已创建普通交互会话，等待用户指令。",
        )
        .await?;

        Ok(session)
    }
}

use a4n_core::error::AppError;
use a4n_db::optimize::OptimizationSessionUpdateInput;

use super::super::{OrchestratorContext, is_cancelled_optimize_error, optimize_db_error};

impl OrchestratorContext {
    pub async fn record_command_error(
        &self,
        session_id: i64,
        phase: &str,
        error_message: &str,
        cancelled: bool,
    ) -> Result<(), AppError> {
        self.save_agent_event(session_id, &format!("{phase}失败或停止：{error_message}"))
            .await?;
        if cancelled {
            return self.mark_session_cancelled_after_stop(session_id).await;
        }
        self.release_in_progress_session_with_message(session_id, phase, error_message)
            .await
    }

    pub(in crate::optimize) async fn release_in_progress_session_after_failure(
        &self,
        session_id: i64,
        phase: &str,
        error: &AppError,
    ) -> Result<(), AppError> {
        if is_cancelled_optimize_error(error) {
            return Ok(());
        }
        self.release_in_progress_session_with_message(session_id, phase, &error.to_string())
            .await
    }

    async fn release_in_progress_session_with_message(
        &self,
        session_id: i64,
        phase: &str,
        error_message: &str,
    ) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        if !matches!(session.status.as_str(), "analyzing" | "fixing") {
            return Ok(());
        }

        a4n_db::optimize::update_optimization_session(
            self.db.connection(),
            OptimizationSessionUpdateInput {
                session,
                status: Some("waiting_user".to_string()),
                issues: None,
                agent_permissions: None,
                agent_instruction: None,
                fixes_applied: None,
                summary: Some(Some(format!("{phase}失败：{error_message}"))),
            },
        )
        .await
        .map_err(optimize_db_error)?;
        Ok(())
    }

    async fn mark_session_cancelled_after_stop(&self, session_id: i64) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        if session.status == "cancelled" {
            return Ok(());
        }
        self.save_agent_event(session_id, "会话已取消，当前运行不会继续写入结果。")
            .await?;
        self.update_cancelled_session(session).await
    }

    /// 取消会话。
    pub(in crate::optimize) async fn cancel_session(
        &self,
        session_id: i64,
    ) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        self.save_agent_event(session_id, "会话已取消，当前运行不会继续写入结果。")
            .await?;
        self.update_cancelled_session(session).await
    }

    async fn update_cancelled_session(
        &self,
        session: a4n_core::models::ai::optimization_session::Model,
    ) -> Result<(), AppError> {
        a4n_db::optimize::update_optimization_session(
            self.db.connection(),
            OptimizationSessionUpdateInput {
                session,
                status: Some("cancelled".to_string()),
                issues: None,
                agent_permissions: None,
                agent_instruction: None,
                fixes_applied: None,
                summary: None,
            },
        )
        .await
        .map_err(optimize_db_error)
        .map(|_| ())
    }
}

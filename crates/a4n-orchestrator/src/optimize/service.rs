use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_session, optimization_session_message};
use a4n_core::models::chapter::chapter;
use a4n_db::optimize::{OptimizationSessionMessageInsertInput, OptimizeDbError};

use super::OrchestratorContext;
use super::runtime_tools::optimization_runtime_tool_specs;

pub(in crate::optimize) fn optimize_db_error(error: OptimizeDbError) -> AppError {
    match error {
        OptimizeDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        OptimizeDbError::Database(error) => AppError::Database(error),
    }
}

impl OrchestratorContext {
    pub async fn optimization_sessions(
        &self,
        project_id: i64,
        chapter_id: Option<i64>,
    ) -> Result<Vec<optimization_session::Model>, AppError> {
        a4n_db::optimize::optimization_sessions(self.db.connection(), project_id, chapter_id)
            .await
            .map_err(optimize_db_error)
    }

    pub async fn optimization_session_by_id(
        &self,
        session_id: i64,
    ) -> Result<Option<optimization_session::Model>, AppError> {
        a4n_db::optimize::optimization_session_by_id(self.db.connection(), session_id)
            .await
            .map_err(optimize_db_error)
    }

    pub async fn insert_optimization_session_message(
        &self,
        input: OptimizationSessionMessageInsertInput,
    ) -> Result<optimization_session_message::Model, AppError> {
        a4n_db::optimize::insert_optimization_session_message(self.db.connection(), input)
            .await
            .map_err(optimize_db_error)
    }

    pub async fn delete_optimization_session_with_messages(
        &self,
        session_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::optimize::delete_optimization_session_with_messages(
            self.db.connection(),
            session_id,
        )
        .await
        .map_err(optimize_db_error)
    }

    /// 提供给提示词的运行时工具清单（OpenAI function calling 格式）。
    pub(in crate::optimize) fn tool_specs(&self) -> Vec<serde_json::Value> {
        optimization_runtime_tool_specs()
    }

    /// 获取会话及消息
    pub(super) async fn get_session(
        &self,
        session_id: i64,
    ) -> Result<optimization_session::Model, AppError> {
        self.optimization_session_by_id(session_id)
            .await?
            .ok_or_else(|| AppError::NotFound("优化会话不存在".into()))
    }

    /// 获取会话消息
    pub(super) async fn session_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_session_message::Model>, AppError> {
        a4n_db::optimize::optimization_session_messages(self.db.connection(), session_id)
            .await
            .map_err(optimize_db_error)
    }

    pub(in crate::optimize) async fn load_optimize_chapter_for_operation(
        &self,
        session: &optimization_session::Model,
        operation: &str,
        deleted_message: &str,
        empty_content_message: Option<&str>,
    ) -> Result<chapter::Model, AppError> {
        let chapter =
            a4n_db::optimize::optimization_chapter_by_id(self.db.connection(), session.chapter_id)
                .await
                .map_err(optimize_db_error)?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
        if chapter.status == "deleted" {
            return Err(AppError::InvalidArgument(deleted_message.to_string()));
        }
        if chapter.status == "generating" {
            return Err(AppError::InvalidState {
                current: chapter.status,
                operation: operation.to_string(),
            });
        }
        if let Some(message) = empty_content_message
            && chapter.content.trim().is_empty()
        {
            return Err(AppError::InvalidArgument(message.to_string()));
        }
        Ok(chapter)
    }
}

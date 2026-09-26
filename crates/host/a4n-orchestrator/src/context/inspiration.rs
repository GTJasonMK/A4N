//! 灵感对话领域私有 owner。
//!
//! 持有 `DbHandle`，封装消息历史、项目上下文读与对话往返落库。
//! LLM 流式编排仍在 `inspiration/` 模块，经本 owner 读写。

use a4n_db::DbHandle;
use a4n_db::DbTransaction;
use a4n_db::inspiration::{
    InspirationDbError, InspirationProjectContextRows, ensure_inspiration_conversation,
    insert_inspiration_exchange_in_transaction,
};
use a4n_db_models::models::user::message;
use a4n_domain::inspiration::InspirationExchangeInsertInput;
use a4n_error::AppError;

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct InspirationApplication {
    database: DbHandle,
}

impl InspirationApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &DbHandle {
        &self.database
    }

    async fn inspiration_messages(&self, project_id: i64) -> Result<Vec<message::Model>, AppError> {
        a4n_db::inspiration::inspiration_messages(self.connection(), project_id, None)
            .await
            .map_err(inspiration_db_error_to_app)
    }

    async fn ensure_inspiration_conversation(&self, project_id: i64) -> Result<(), AppError> {
        ensure_inspiration_conversation(self.connection(), project_id)
            .await
            .map_err(inspiration_db_error_to_app)
    }

    async fn inspiration_project_context_rows(
        &self,
        project_id: i64,
    ) -> Result<InspirationProjectContextRows, AppError> {
        a4n_db::inspiration::inspiration_project_context_rows(self.connection(), project_id)
            .await
            .map_err(inspiration_db_error_to_app)
    }

    async fn save_inspiration_exchange_in_transaction(
        &self,
        tx: &DbTransaction,
        project_id: i64,
        user_input: String,
        assistant_output: String,
    ) -> Result<i64, AppError> {
        insert_inspiration_exchange_in_transaction(
            tx,
            InspirationExchangeInsertInput {
                project_id,
                user_input,
                assistant_output,
            },
        )
        .await
        .map_err(inspiration_db_error_to_app)
    }
}

pub(crate) fn inspiration_db_error_to_app(error: InspirationDbError) -> AppError {
    match error {
        InspirationDbError::NotFound(message) => AppError::NotFound(message),
        InspirationDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        InspirationDbError::Database(error) => AppError::database(error),
    }
}

impl OrchestratorContext {
    pub(crate) async fn ensure_inspiration_conversation(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        self.inspiration
            .ensure_inspiration_conversation(project_id)
            .await
    }

    pub(crate) async fn inspiration_messages(
        &self,
        project_id: i64,
    ) -> Result<Vec<message::Model>, AppError> {
        self.inspiration.inspiration_messages(project_id).await
    }

    pub(crate) async fn inspiration_project_context_rows(
        &self,
        project_id: i64,
    ) -> Result<InspirationProjectContextRows, AppError> {
        self.inspiration
            .inspiration_project_context_rows(project_id)
            .await
    }

    /// 仅供 inspiration 子树编排使用。
    pub(crate) async fn save_inspiration_exchange_in_transaction(
        &self,
        tx: &DbTransaction,
        project_id: i64,
        user_input: &str,
        assistant_output: &str,
    ) -> Result<i64, AppError> {
        use crate::inspiration::required_message_content;
        let user_input = required_message_content(user_input)?;
        let assistant_output = required_message_content(assistant_output)?;
        self.inspiration
            .save_inspiration_exchange_in_transaction(
                tx,
                project_id,
                user_input.to_string(),
                assistant_output.to_string(),
            )
            .await
    }

    /// 灵感对话：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_inspiration_workflow_validated(
        &self,
        project_id: i64,
        mode: &str,
        request: &a4n_llm_provider_contract::provider::ChatRequest,
        active_task_id: Option<String>,
    ) -> Result<a4n_db_models::models::generation::workflow_run::Model, AppError> {
        let input = crate::inspiration::inspiration_workflow_run_input(
            project_id,
            mode,
            request,
            active_task_id,
        )?;
        self.begin_workflow_run_validated(input).await
    }
}

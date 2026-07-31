//! 导入反推领域私有 owner。
//!
//! 持有 `DbHandle`，封装反推上下文读取与蓝图/大纲/分部大纲落库事务。
//! JSON 合同校验与 apply 结果投影仍在 `import_reverse/application`。

use a4n_core::error::AppError;
use a4n_db::DbHandle;
use a4n_db::import_reverse::{
    ImportReverseDbError, ReverseBlueprintApplyDraft, ReverseContextRows, ReverseOutlineApplyRow,
    ReversePartOutlineApplyRow,
};

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct ImportReverseApplication {
    database: DbHandle,
}

impl ImportReverseApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn reverse_context_rows(
        &self,
        project_id: i64,
        include_blueprint_context: bool,
    ) -> Result<ReverseContextRows, AppError> {
        a4n_db::import_reverse::reverse_context_rows(
            self.connection(),
            project_id,
            include_blueprint_context,
        )
        .await
        .map_err(import_reverse_db_error)
    }

    async fn apply_reverse_blueprint_records_transaction(
        &self,
        project_id: i64,
        draft: ReverseBlueprintApplyDraft,
    ) -> Result<a4n_db::import_reverse::ReverseBlueprintApplyResult, AppError> {
        a4n_db::import_reverse::apply_reverse_blueprint_records_transaction(
            self.connection(),
            project_id,
            draft,
        )
        .await
        .map_err(import_reverse_db_error)
    }

    async fn apply_reverse_outline_records_transaction(
        &self,
        project_id: i64,
        rows: &[ReverseOutlineApplyRow],
    ) -> Result<usize, AppError> {
        a4n_db::import_reverse::apply_reverse_outline_records_transaction(
            self.connection(),
            project_id,
            rows,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn apply_reverse_part_outline_records_transaction(
        &self,
        project_id: i64,
        rows: &[ReversePartOutlineApplyRow],
    ) -> Result<usize, AppError> {
        a4n_db::import_reverse::apply_reverse_part_outline_records_transaction(
            self.connection(),
            project_id,
            rows,
        )
        .await
        .map_err(AppError::Database)
    }
}

fn import_reverse_db_error(error: ImportReverseDbError) -> AppError {
    match error {
        ImportReverseDbError::Database(error) => AppError::Database(error),
        ImportReverseDbError::NotFound(message) => AppError::NotFound(message),
        ImportReverseDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
    }
}

impl OrchestratorContext {
    pub(crate) async fn reverse_context_rows(
        &self,
        project_id: i64,
        include_blueprint_context: bool,
    ) -> Result<ReverseContextRows, AppError> {
        self.import_reverse
            .reverse_context_rows(project_id, include_blueprint_context)
            .await
    }

    /// 仅供 import_reverse 编排使用。
    pub(crate) async fn apply_reverse_blueprint_records_transaction(
        &self,
        project_id: i64,
        draft: ReverseBlueprintApplyDraft,
    ) -> Result<a4n_db::import_reverse::ReverseBlueprintApplyResult, AppError> {
        self.import_reverse
            .apply_reverse_blueprint_records_transaction(project_id, draft)
            .await
    }

    /// 仅供 import_reverse 编排使用。
    pub(crate) async fn apply_reverse_outline_records_transaction(
        &self,
        project_id: i64,
        rows: &[ReverseOutlineApplyRow],
    ) -> Result<usize, AppError> {
        self.import_reverse
            .apply_reverse_outline_records_transaction(project_id, rows)
            .await
    }

    /// 仅供 import_reverse 编排使用。
    pub(crate) async fn apply_reverse_part_outline_records_transaction(
        &self,
        project_id: i64,
        rows: &[ReversePartOutlineApplyRow],
    ) -> Result<usize, AppError> {
        self.import_reverse
            .apply_reverse_part_outline_records_transaction(project_id, rows)
            .await
    }

    /// 导入逆向工程：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_import_reverse_workflow_validated(
        &self,
        project_id: i64,
        prompt_name: &str,
        apply: bool,
        content_sample: &str,
        auto_context: bool,
        context_limit: Option<i64>,
        variables: &std::collections::HashMap<String, serde_json::Value>,
        active_task_id: Option<&str>,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let input = crate::import_reverse::import_reverse_workflow_run_input(
            project_id,
            prompt_name,
            apply,
            content_sample,
            auto_context,
            context_limit,
            variables,
            active_task_id,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }
}

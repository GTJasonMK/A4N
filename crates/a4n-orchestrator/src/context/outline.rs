//! 大纲生成领域私有 owner。
//!
//! 持有 `DbHandle`，供分部/章节大纲生成、同步与删除等写路径使用。
//! 生成编排（LLM/workflow）仍在 `outline/` 模块，但数据库只经本 owner。

use a4n_core::error::AppError;
use a4n_db::DbHandle;
use serde_json::Value;

use super::OrchestratorContext;
use crate::outline::outline_db_error;

/// 大纲领域 application owner。
#[derive(Clone)]
pub(super) struct OutlineApplication {
    database: DbHandle,
}

impl OutlineApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn delete_latest_part_outline_records(
        &self,
        project_id: i64,
        count: u64,
    ) -> Result<(), AppError> {
        a4n_db::outline::delete_latest_part_outline_records(
            self.database.connection(),
            project_id,
            count,
        )
        .await
        .map_err(outline_db_error)
    }
}

impl OrchestratorContext {
    /// 删除项目最后 N 个分部大纲及其范围内章节大纲。
    pub async fn delete_latest_part_outline_records_validated(
        &self,
        project_id: i64,
        count: u64,
    ) -> Result<(), AppError> {
        crate::project::ensure_positive_project_id(project_id)?;
        if count == 0 {
            return Err(AppError::InvalidArgument(
                "删除分部数量必须大于 0".to_string(),
            ));
        }
        self.outline
            .delete_latest_part_outline_records(project_id, count)
            .await
    }

    /// 大纲 application 内部创建/恢复 workflow_run（RUNNING）。
    pub(crate) async fn begin_outline_workflow_validated(
        &self,
        project_id: i64,
        workflow_kind: &str,
        mode: &str,
        current_step_key: &str,
        task_id: Option<String>,
        input_fingerprint_json: Value,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let input = crate::outline::outline_workflow_run_input(
            project_id,
            workflow_kind,
            mode,
            current_step_key,
            task_id,
            input_fingerprint_json,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }
}

//! 导入反推领域私有 owner。
//!
//! 持有 `DbHandle`，封装反推上下文读取与蓝图/大纲/分部大纲落库事务。
//! JSON 合同校验与 apply 结果投影仍在 `import_reverse/application`。

use a4n_db::DbHandle;
use a4n_db::import_reverse::ReverseContextRows;
use a4n_error::AppError;

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct ImportReverseApplication {
    database: DbHandle,
}

impl ImportReverseApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &a4n_db::DbHandle {
        &self.database
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
        .map_err(crate::import_reverse::import_reverse_db_error)
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
}

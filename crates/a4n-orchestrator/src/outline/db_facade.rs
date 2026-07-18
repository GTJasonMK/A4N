//! 大纲分部/章节大纲的 orchestrator facade。

use crate::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_db::outline::OutlineDbError;

impl OrchestratorContext {
    /// 删除项目最后 N 个分部大纲及其范围内章节大纲。
    pub async fn delete_latest_part_outline_records(
        &self,
        project_id: i64,
        count: u64,
    ) -> Result<(), AppError> {
        a4n_db::outline::delete_latest_part_outline_records(self.db.connection(), project_id, count)
            .await
            .map_err(outline_db_error_to_app_error)
    }
}

fn outline_db_error_to_app_error(error: OutlineDbError) -> AppError {
    match error {
        OutlineDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        OutlineDbError::Database(error) => AppError::Database(error),
        OutlineDbError::Generation(error) => crate::generation::generation_error(error),
    }
}

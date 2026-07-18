use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use a4n_db::project as project_db;

use crate::outline::OrchestratorContext;

impl OrchestratorContext {
    /// 获取分部大纲详情
    pub async fn get_part(&self, part_id: i64) -> Result<part_outline::Model, AppError> {
        project_db::part_outline_by_id(self.db.connection(), part_id)
            .await
            .map_err(crate::project::project_outline_record_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("分部大纲不存在: {part_id}")))
    }
}

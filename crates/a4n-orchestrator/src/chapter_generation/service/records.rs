use super::ChapterGenerationService;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;

impl ChapterGenerationService {
    /// 获取项目所有章节（按序号排序）
    pub async fn list(&self, project_id: i64) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::chapter_content::chapters_for_project(self.db.as_ref(), project_id)
            .await
            .map_err(AppError::Database)
    }

    /// 获取章节详情
    pub async fn get(&self, chapter_id: i64) -> Result<chapter::Model, AppError> {
        a4n_db::project::chapter_by_id(self.db.as_ref(), chapter_id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))
    }
}

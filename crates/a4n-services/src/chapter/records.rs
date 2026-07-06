use super::ChapterService;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use sea_orm::*;

impl ChapterService {
    /// 获取项目所有章节（按序号排序）
    pub async fn list(&self, project_id: i64) -> Result<Vec<chapter::Model>, AppError> {
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .order_by_asc(chapter::Column::ChapterNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 获取章节详情
    pub async fn get(&self, chapter_id: i64) -> Result<chapter::Model, AppError> {
        chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))
    }
}

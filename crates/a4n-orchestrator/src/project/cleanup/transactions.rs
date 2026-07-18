use a4n_core::{error::AppError, models::chapter::chapter};
use sea_orm::TransactionTrait;

use super::super::{ChapterDerivedCleanupArtifacts, ChapterDerivedCleanupResult};
use super::{cleanup_chapter_derived_data_in, delete_chapter_content_record_in};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn cleanup_chapter_derived_data_record<F>(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        include_generated_outputs: bool,
        validate_artifacts: F,
    ) -> Result<ChapterDerivedCleanupArtifacts, AppError>
    where
        F: FnOnce(&ChapterDerivedCleanupArtifacts) -> Result<(), AppError>,
    {
        let tx = self
            .db
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let artifacts = cleanup_chapter_derived_data_in(
            &tx,
            project_id,
            chapter_id,
            chapter_number,
            include_generated_outputs,
        )
        .await?;
        if let Err(error) = validate_artifacts(&artifacts) {
            if let Err(rollback_error) = tx.rollback().await {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "章节派生数据清理校验失败：{error}；事务回滚失败：{rollback_error}"
                )));
            }
            return Err(error);
        }
        tx.commit().await.map_err(AppError::Database)?;
        Ok(artifacts)
    }

    pub async fn delete_chapter_content_records_and_outlines_record<F>(
        &self,
        project_id: i64,
        chapters: Vec<chapter::Model>,
        outline_ids: &[i64],
        validate_results: F,
    ) -> Result<Vec<ChapterDerivedCleanupResult>, AppError>
    where
        F: FnOnce(&[ChapterDerivedCleanupResult]) -> Result<(), AppError>,
    {
        let tx = self
            .db
            .connection()
            .begin()
            .await
            .map_err(AppError::Database)?;
        let mut cleanup_results = Vec::with_capacity(chapters.len());
        for chapter in chapters {
            let artifacts = delete_chapter_content_record_in(
                &tx,
                project_id,
                chapter.id,
                chapter.chapter_number,
            )
            .await?;
            cleanup_results.push(ChapterDerivedCleanupResult {
                chapter_id: chapter.id,
                chapter_number: chapter.chapter_number,
                artifacts,
            });
        }
        if let Err(error) = validate_results(&cleanup_results) {
            if let Err(rollback_error) = tx.rollback().await {
                return Err(AppError::Internal(anyhow::anyhow!(
                    "章节内容清理校验失败：{error}；事务回滚失败：{rollback_error}"
                )));
            }
            return Err(error);
        }
        a4n_db::project::delete_chapter_outlines_by_ids(&tx, outline_ids)
            .await
            .map_err(crate::errors::project_cleanup_error_to_app_error)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(cleanup_results)
    }
}

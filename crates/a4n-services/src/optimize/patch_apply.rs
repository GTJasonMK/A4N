use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};

use super::OptimizeService;
use super::patch::{
    OptimizationPatch, OptimizationPatchApplyResult, apply_optimization_patches_to_content,
    optimization_content_hash,
};

impl OptimizeService {
    pub async fn write_patches_to_session_chapter(
        &self,
        session_id: i64,
        base_content_hash: &str,
        patches: &[OptimizationPatch],
    ) -> Result<OptimizationPatchApplyResult, AppError> {
        if patches.is_empty() {
            return Err(AppError::InvalidArgument(
                "应用补丁列表不能为空".to_string(),
            ));
        }

        let session = self.get_session(session_id).await?;
        let chapter = self
            .load_optimize_chapter_for_operation(
                &session,
                "应用局部补丁",
                "已删除章节不能应用局部补丁",
                None,
            )
            .await?;

        let next_content =
            apply_optimization_patches_to_content(&chapter.content, base_content_hash, patches)?;
        let word_count = next_content.chars().count() as i32;
        let mut active: chapter::ActiveModel = chapter.into();
        active.content = Set(next_content);
        active.word_count = Set(word_count);
        active.status = Set("draft".to_string());
        active.summary = Set(None);
        active.style_tags = Set(None);
        active.analysis_data = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let content_hash = optimization_content_hash(&updated.content);

        Ok(OptimizationPatchApplyResult {
            session_id,
            chapter_id: updated.id,
            applied_count: patches.len(),
            word_count: updated.word_count,
            content_hash,
        })
    }
}

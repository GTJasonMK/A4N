use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use a4n_db::project as project_db;

use crate::outline::OrchestratorContext;

impl OrchestratorContext {
    pub(super) async fn existing_chapter_outline_for_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        let outlines = project_db::chapter_outlines_for_project_chapter_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(crate::project::project_outline_record_error_to_app_error)?;
        if outlines.len() > 1 {
            let ids = outlines
                .iter()
                .map(|outline| outline.id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AppError::InvalidArgument(format!(
                "第 {chapter_number} 章已有重复章纲记录：{ids}，请先清理后再生成"
            )));
        }
        Ok(outlines.into_iter().next())
    }

    pub(super) async fn ensure_no_other_outline_for_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline_id: i64,
    ) -> Result<(), AppError> {
        let conflicts = project_db::chapter_outlines_for_project_chapter_number(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(crate::project::project_outline_record_error_to_app_error)?
        .into_iter()
        .filter(|outline| outline.id != outline_id)
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
        if conflicts.is_empty() {
            return Ok(());
        }

        Err(AppError::InvalidArgument(format!(
            "第 {chapter_number} 章存在其他章纲记录：{}",
            conflicts
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

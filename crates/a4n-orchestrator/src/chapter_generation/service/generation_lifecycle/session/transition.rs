use a4n_error::AppError;
use chrono::Utc;

use super::super::chapter_generation_lifecycle_error_to_app_error;
use super::{ChapterGenerationSession, ExistingChapterGeneration};
use crate::chapter_generation::service::ChapterGenerationService;

impl ChapterGenerationService {
    pub(in crate::chapter_generation) async fn begin_generation(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<ChapterGenerationSession, AppError> {
        let guard = self.try_generation_guard(project_id, chapter_number)?;
        let existing_chapter = a4n_db::chapter_content::begin_chapter_generation_record(
            self.db.as_ref(),
            project_id,
            chapter_number,
            Utc::now().naive_utc(),
        )
        .await
        .map_err(chapter_generation_lifecycle_error_to_app_error)?
        .map(ExistingChapterGeneration::from);
        Ok(ChapterGenerationSession {
            project_id,
            chapter_number,
            existing_chapter,
            _guard: guard,
        })
    }

    pub(in crate::chapter_generation) async fn restore_generation_status_ref(
        &self,
        session: &ChapterGenerationSession,
    ) {
        let Some(existing_chapter) = session.existing_chapter.as_ref() else {
            return;
        };

        if let Err(err) = a4n_db::chapter_content::restore_chapter_generation_status_record(
            self.db.as_ref(),
            existing_chapter.id,
            existing_chapter.previous_status.clone(),
            Utc::now().naive_utc(),
        )
        .await
        {
            tracing::warn!(
                chapter_id = existing_chapter.id,
                error = %err,
                "章节生成失败后恢复状态失败"
            );
        }
    }
}

use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;

use crate::OrchestratorContext;

use super::{ChapterVideoPromptOptions, errors::video_db_error};

impl OrchestratorContext {
    pub(crate) async fn get_chapter(&self, chapter_id: i64) -> Result<ChapterRecord, AppError> {
        a4n_db::video::video_chapter_domain_record(self.database.executor(), chapter_id)
            .await
            .map_err(video_db_error)
    }

    pub(super) async fn ensure_chapter_belongs_to_project(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::ensure_video_chapter_belongs_to_project_record(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)
    }

    pub(super) async fn ensure_prompt_generation_inputs(
        &self,
        project_id: i64,
        chapter_id: i64,
        options: &ChapterVideoPromptOptions,
    ) -> Result<(), AppError> {
        let reference_ids_json = serde_json::json!(options.reference_image_ids);
        let (chapter_result, references_result) = futures::join!(
            self.ensure_chapter_belongs_to_project(project_id, chapter_id),
            self.ensure_video_reference_images_available(
                project_id,
                chapter_id,
                Some(&reference_ids_json),
            ),
        );
        chapter_result?;
        references_result
    }
}

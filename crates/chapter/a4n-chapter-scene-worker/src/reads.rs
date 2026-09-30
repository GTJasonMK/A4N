use super::*;
use a4n_chapter_scene_worker_contract::CHAPTER_SCENE_LIST_OPERATION;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ChapterInput>,
    ) -> Result<Vec<ChapterSceneSummary>, WorkerFailure> {
        validate_request(&request, CHAPTER_SCENE_LIST_OPERATION)?;
        let chapter = self
            .authorized_chapter(request.payload.chapter_id, user_id(&request)?)
            .await?;
        let scenes =
            a4n_db::project::chapter_scenes_by_chapter(self.database.connection(), chapter.id)
                .await
                .map_err(record_error)?;
        Ok(scenes.into_iter().map(scene_summary).collect())
    }
}

use a4n_core::{error::AppError, models::chapter::chapter_scene};
use a4n_db::project::ProjectChapterSceneReorderInput;
use a4n_orchestrator::chapter_scene::{
    ChapterSceneGenerationApplicationInput, ChapterSceneProgressReporter,
    ChapterSceneRequestPreview, ChapterSceneTaskScope,
};
use a4n_orchestrator::{OrchestratorContext, project::ChapterSceneUpdatePatch};
use a4n_prompt::rendering::PromptRenderer;

pub(crate) struct ProjectSceneRecordsRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> ProjectSceneRecordsRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn chapter_scenes_for_user_validated(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.orchestrator
            .chapter_scenes_for_user_validated(user_id, chapter_id)
            .await
    }

    pub(crate) async fn update_chapter_scene_for_user_validated(
        &self,
        user_id: i64,
        scene_id: i64,
        patch: ChapterSceneUpdatePatch,
    ) -> Result<chapter_scene::Model, AppError> {
        self.orchestrator
            .update_chapter_scene_for_user_validated(user_id, scene_id, patch)
            .await
    }

    pub(crate) async fn delete_chapter_scene_for_user_validated(
        &self,
        user_id: i64,
        scene_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_chapter_scene_for_user_validated(user_id, scene_id)
            .await
    }

    pub(crate) async fn reorder_chapter_scenes_for_user_validated(
        &self,
        user_id: i64,
        input: ProjectChapterSceneReorderInput,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.orchestrator
            .reorder_chapter_scenes_for_user_validated(user_id, input)
            .await
    }

    pub(crate) async fn chapter_scene_task_scope_for_user_validated(
        &self,
        user_id: i64,
        input: &ChapterSceneGenerationApplicationInput,
    ) -> Result<ChapterSceneTaskScope, AppError> {
        self.orchestrator
            .chapter_scene_task_scope_for_user_validated(user_id, input)
            .await
    }

    pub(crate) async fn prepare_chapter_scene_request_preview_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterSceneGenerationApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChapterSceneRequestPreview, AppError> {
        self.orchestrator
            .prepare_chapter_scene_request_preview_for_user_validated(
                user_id,
                input,
                prompt_renderer,
            )
            .await
    }

    pub(crate) async fn generate_chapter_scene_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterSceneGenerationApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: ChapterSceneProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<chapter_scene::Model, AppError> {
        self.orchestrator
            .generate_chapter_scene_for_user_validated(
                user_id,
                input,
                prompt_renderer,
                progress,
                active_task_id,
            )
            .await
    }
}

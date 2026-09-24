use a4n_db::DbHandle;
use a4n_db_models::models::chapter::chapter_scene;
use a4n_domain::chapter::ProjectGeneratedChapterSceneSaveInput;
use a4n_error::AppError;

use super::super::OrchestratorContext;
use crate::project::{
    project_chapter_scene_record_error_to_app_error, project_chapter_scene_write_error_to_app_error,
};

#[derive(Clone)]
pub(in crate::context) struct ProjectSceneRecordsApplication {
    database: DbHandle,
}

impl ProjectSceneRecordsApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn scenes_by_chapter(
        &self,
        chapter_id: i64,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        a4n_db::project::chapter_scenes_by_chapter(self.database.connection(), chapter_id)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    async fn save_generated_scene(
        &self,
        input: ProjectGeneratedChapterSceneSaveInput,
    ) -> Result<chapter_scene::Model, AppError> {
        a4n_db::project::save_generated_chapter_scene_record(self.database.connection(), input)
            .await
            .map_err(project_chapter_scene_write_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn chapter_scenes_by_chapter(
        &self,
        chapter_id: i64,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.project_scene_records
            .scenes_by_chapter(chapter_id)
            .await
    }

    pub(crate) async fn save_generated_chapter_scene_record(
        &self,
        input: ProjectGeneratedChapterSceneSaveInput,
    ) -> Result<chapter_scene::Model, AppError> {
        self.project_scene_records.save_generated_scene(input).await
    }
}

fn chapter_scene_summary(
    scene: chapter_scene::Model,
) -> a4n_api_contract::chapter::ChapterSceneSummary {
    a4n_api_contract::chapter::ChapterSceneSummary {
        id: scene.id,
        chapter_id: scene.chapter_id,
        scene_number: scene.scene_number,
        title: scene.title,
        content: scene.content,
        word_count: scene.word_count,
        summary: scene.summary,
        scene_type: scene.scene_type,
        characters_present: scene.characters_present,
        location: scene.location,
        timeline_position: scene.timeline_position,
        transition_from_previous: scene.transition_from_previous,
        created_at: scene.created_at.to_string(),
        updated_at: scene.updated_at.to_string(),
    }
}

#[async_trait::async_trait]
impl crate::chapter_scene::ChapterSceneGenerationApplicationPort for OrchestratorContext {
    async fn project_id_for_user(
        &self,
        user_id: i64,
        input: &a4n_api_contract::chapter::GenerateChapterSceneInput,
    ) -> Result<i64, AppError> {
        self.chapter_scene_project_id_for_user_validated(user_id, input)
            .await
    }

    async fn preview_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter::GenerateChapterSceneInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
    ) -> Result<crate::chapter_scene::ChapterSceneRequestPreview, AppError> {
        self.prepare_chapter_scene_request_preview_for_user_validated(
            user_id,
            input,
            prompt_renderer,
        )
        .await
    }

    async fn generate_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter::GenerateChapterSceneInput,
        prompt_renderer: &dyn a4n_prompt::rendering::PromptRenderer,
        progress: a4n_api_contract::chapter::ChapterSceneProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<a4n_api_contract::chapter::ChapterSceneSummary, AppError> {
        Ok(chapter_scene_summary(
            self.generate_chapter_scene_for_user_validated(
                user_id,
                input,
                prompt_renderer,
                progress,
                active_task_id,
            )
            .await?,
        ))
    }
}

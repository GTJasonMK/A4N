use a4n_api_contract::chapter::{ReorderChapterScenesInput, UpdateChapterSceneInput};
use a4n_db::{
    DbHandle,
    project::{ProjectChapterSceneUpdateInput, ProjectGeneratedChapterSceneSaveInput},
};
use a4n_db_models::models::chapter::chapter_scene;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::project::{
    chapter_scene_update_record_input, project_chapter_scene_record_error_to_app_error,
    project_chapter_scene_write_error_to_app_error,
};

#[derive(Clone)]
pub(super) struct ProjectSceneRecordsApplication {
    database: DbHandle,
}

impl ProjectSceneRecordsApplication {
    pub(super) fn new(database: DbHandle) -> Self {
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

    async fn scene_by_id(&self, scene_id: i64) -> Result<Option<chapter_scene::Model>, AppError> {
        a4n_db::project::chapter_scene_by_id(self.database.connection(), scene_id)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    async fn update_scene(
        &self,
        input: ProjectChapterSceneUpdateInput,
    ) -> Result<chapter_scene::Model, AppError> {
        a4n_db::project::update_chapter_scene_record(self.database.connection(), input)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    async fn delete_scene(&self, scene_id: i64) -> Result<u64, AppError> {
        a4n_db::project::delete_chapter_scene_record(self.database.connection(), scene_id)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    async fn reorder_project_id(&self, order: &[(i64, i32)]) -> Result<i64, AppError> {
        a4n_db::project::chapter_scene_reorder_project_id(self.database.connection(), order)
            .await
            .map_err(project_chapter_scene_write_error_to_app_error)
    }

    async fn reorder_scenes(
        &self,
        order: &[(i64, i32)],
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        a4n_db::project::reorder_chapter_scene_records(self.database.connection(), order)
            .await
            .map_err(project_chapter_scene_write_error_to_app_error)
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

    pub(crate) async fn chapter_scenes_for_user_validated(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.chapter_for_editor_validated(user_id, chapter_id)
            .await?;
        self.chapter_scenes_by_chapter(chapter_id).await
    }

    pub(crate) async fn chapter_scene_by_id(
        &self,
        scene_id: i64,
    ) -> Result<Option<chapter_scene::Model>, AppError> {
        self.project_scene_records.scene_by_id(scene_id).await
    }

    async fn update_chapter_scene_validated(
        &self,
        scene: chapter_scene::Model,
        input: UpdateChapterSceneInput,
    ) -> Result<chapter_scene::Model, AppError> {
        let input =
            chapter_scene_update_record_input(scene, input, chrono::Utc::now().naive_utc())?;
        self.project_scene_records.update_scene(input).await
    }

    pub(crate) async fn update_chapter_scene_for_user_validated(
        &self,
        user_id: i64,
        input: UpdateChapterSceneInput,
    ) -> Result<chapter_scene::Model, AppError> {
        let scene_id = input.scene_id;
        let scene = self
            .chapter_scene_by_id(scene_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("场景不存在: scene_id={scene_id}")))?;
        self.chapter_for_editor_validated(user_id, scene.chapter_id)
            .await?;
        self.update_chapter_scene_validated(scene, input).await
    }

    pub(crate) async fn delete_chapter_scene_validated(
        &self,
        scene_id: i64,
    ) -> Result<(), AppError> {
        let rows_affected = self.project_scene_records.delete_scene(scene_id).await?;
        if rows_affected == 0 {
            return Err(AppError::NotFound(format!(
                "场景不存在: scene_id={scene_id}"
            )));
        }
        Ok(())
    }

    pub(crate) async fn delete_chapter_scene_for_user_validated(
        &self,
        user_id: i64,
        scene_id: i64,
    ) -> Result<(), AppError> {
        let scene = self
            .chapter_scene_by_id(scene_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("场景不存在: scene_id={scene_id}")))?;
        self.chapter_for_editor_validated(user_id, scene.chapter_id)
            .await?;
        self.delete_chapter_scene_validated(scene_id).await
    }

    async fn chapter_scene_reorder_project_id_validated(
        &self,
        order: &[(i64, i32)],
    ) -> Result<i64, AppError> {
        self.project_scene_records.reorder_project_id(order).await
    }

    async fn reorder_chapter_scenes_validated(
        &self,
        order: &[(i64, i32)],
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.project_scene_records.reorder_scenes(order).await
    }

    pub(crate) async fn reorder_chapter_scenes_for_user_validated(
        &self,
        user_id: i64,
        input: ReorderChapterScenesInput,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        let project_id = self
            .chapter_scene_reorder_project_id_validated(&input.order)
            .await?;
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.reorder_chapter_scenes_validated(&input.order).await
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
impl a4n_api_contract::chapter::ChapterSceneReadApplicationPort for OrchestratorContext {
    async fn list(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<a4n_api_contract::chapter::ChapterSceneSummary>, AppError> {
        Ok(
            OrchestratorContext::chapter_scenes_for_user_validated(self, user_id, chapter_id)
                .await?
                .into_iter()
                .map(chapter_scene_summary)
                .collect(),
        )
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter::ChapterSceneMutationApplicationPort for OrchestratorContext {
    async fn update(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter::UpdateChapterSceneInput,
    ) -> Result<a4n_api_contract::chapter::ChapterSceneSummary, AppError> {
        Ok(chapter_scene_summary(
            OrchestratorContext::update_chapter_scene_for_user_validated(self, user_id, input)
                .await?,
        ))
    }

    async fn delete(&self, user_id: i64, scene_id: i64) -> Result<(), AppError> {
        OrchestratorContext::delete_chapter_scene_for_user_validated(self, user_id, scene_id).await
    }

    async fn reorder(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter::ReorderChapterScenesInput,
    ) -> Result<Vec<a4n_api_contract::chapter::ChapterSceneSummary>, AppError> {
        Ok(
            OrchestratorContext::reorder_chapter_scenes_for_user_validated(self, user_id, input)
                .await?
                .into_iter()
                .map(chapter_scene_summary)
                .collect(),
        )
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

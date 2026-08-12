use a4n_api_contract::chapter::{ReorderChapterScenesInput, UpdateChapterSceneInput};
use a4n_core::{error::AppError, models::chapter::chapter_scene};
use a4n_db::{
    DbHandle,
    project::{
        ProjectChapterSceneReorderInput, ProjectChapterSceneReorderScope,
        ProjectChapterSceneUpdateInput, ProjectGeneratedChapterSceneSaveInput,
    },
};

use super::OrchestratorContext;
use crate::project::{
    chapter_scene_update_record_input, project_chapter_scene_record_error_to_app_error,
    project_chapter_scene_reorder_error_to_app_error,
    project_generated_chapter_scene_save_error_to_app_error,
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

    async fn reorder_scope(
        &self,
        input: ProjectChapterSceneReorderInput,
    ) -> Result<ProjectChapterSceneReorderScope, AppError> {
        a4n_db::project::chapter_scene_reorder_scope(self.database.connection(), input)
            .await
            .map_err(project_chapter_scene_reorder_error_to_app_error)
    }

    async fn reorder_scenes(
        &self,
        input: ProjectChapterSceneReorderInput,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        a4n_db::project::reorder_chapter_scene_records(self.database.connection(), input)
            .await
            .map_err(project_chapter_scene_reorder_error_to_app_error)
    }

    async fn save_generated_scene(
        &self,
        input: ProjectGeneratedChapterSceneSaveInput,
    ) -> Result<chapter_scene::Model, AppError> {
        a4n_db::project::save_generated_chapter_scene_record(self.database.connection(), input)
            .await
            .map_err(project_generated_chapter_scene_save_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn chapter_scenes_by_chapter(
        &self,
        chapter_id: i64,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.project_scene_records
            .scenes_by_chapter(chapter_id)
            .await
    }

    pub async fn chapter_scenes_for_user_validated(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.chapter_for_editor_validated(user_id, chapter_id)
            .await?;
        self.chapter_scenes_by_chapter(chapter_id).await
    }

    pub async fn chapter_scene_by_id(
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

    pub async fn update_chapter_scene_for_user_validated(
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

    pub async fn delete_chapter_scene_validated(&self, scene_id: i64) -> Result<(), AppError> {
        let rows_affected = self.project_scene_records.delete_scene(scene_id).await?;
        if rows_affected == 0 {
            return Err(AppError::NotFound(format!(
                "场景不存在: scene_id={scene_id}"
            )));
        }
        Ok(())
    }

    pub async fn delete_chapter_scene_for_user_validated(
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

    async fn chapter_scene_reorder_scope_validated(
        &self,
        input: ProjectChapterSceneReorderInput,
    ) -> Result<ProjectChapterSceneReorderScope, AppError> {
        self.project_scene_records.reorder_scope(input).await
    }

    async fn reorder_chapter_scenes_validated(
        &self,
        input: ProjectChapterSceneReorderInput,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        self.project_scene_records.reorder_scenes(input).await
    }

    pub async fn reorder_chapter_scenes_for_user_validated(
        &self,
        user_id: i64,
        input: ReorderChapterScenesInput,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        let input = ProjectChapterSceneReorderInput { order: input.order };
        let scope = self
            .chapter_scene_reorder_scope_validated(input.clone())
            .await?;
        self.verify_project_permission(
            scope.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.reorder_chapter_scenes_validated(input).await
    }

    pub(crate) async fn save_generated_chapter_scene_record(
        &self,
        input: ProjectGeneratedChapterSceneSaveInput,
    ) -> Result<chapter_scene::Model, AppError> {
        self.project_scene_records.save_generated_scene(input).await
    }
}

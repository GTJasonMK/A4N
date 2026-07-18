use a4n_db::project::{
    ProjectChapterSceneReorderInput, ProjectChapterSceneReorderScope,
    ProjectChapterSceneUpdateInput, ProjectGeneratedChapterSceneSaveInput,
};

use super::super::chapter_scene;
use super::super::errors::{
    project_chapter_scene_record_error_to_app_error,
    project_chapter_scene_reorder_error_to_app_error,
    project_generated_chapter_scene_save_error_to_app_error,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn chapter_scenes_by_chapter(
        &self,
        chapter_id: i64,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        a4n_db::project::chapter_scenes_by_chapter(self.db.connection(), chapter_id)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    pub async fn chapter_scene_by_id(
        &self,
        scene_id: i64,
    ) -> Result<Option<chapter_scene::Model>, AppError> {
        a4n_db::project::chapter_scene_by_id(self.db.connection(), scene_id)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    pub async fn update_chapter_scene_record(
        &self,
        input: ProjectChapterSceneUpdateInput,
    ) -> Result<chapter_scene::Model, AppError> {
        a4n_db::project::update_chapter_scene_record(self.db.connection(), input)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    pub async fn delete_chapter_scene_record(&self, scene_id: i64) -> Result<u64, AppError> {
        a4n_db::project::delete_chapter_scene_record(self.db.connection(), scene_id)
            .await
            .map_err(project_chapter_scene_record_error_to_app_error)
    }

    pub async fn chapter_scene_reorder_scope(
        &self,
        input: ProjectChapterSceneReorderInput,
    ) -> Result<ProjectChapterSceneReorderScope, AppError> {
        a4n_db::project::chapter_scene_reorder_scope(self.db.connection(), input)
            .await
            .map_err(project_chapter_scene_reorder_error_to_app_error)
    }

    pub async fn reorder_chapter_scene_records(
        &self,
        input: ProjectChapterSceneReorderInput,
    ) -> Result<Vec<chapter_scene::Model>, AppError> {
        a4n_db::project::reorder_chapter_scene_records(self.db.connection(), input)
            .await
            .map_err(project_chapter_scene_reorder_error_to_app_error)
    }

    pub async fn save_generated_chapter_scene_record(
        &self,
        input: ProjectGeneratedChapterSceneSaveInput,
    ) -> Result<chapter_scene::Model, AppError> {
        a4n_db::project::save_generated_chapter_scene_record(self.db.connection(), input)
            .await
            .map_err(project_generated_chapter_scene_save_error_to_app_error)
    }
}

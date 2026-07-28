use std::collections::BTreeMap;

use a4n_core::error::AppError;
use a4n_orchestrator::{
    OrchestratorContext,
    scene_profile::{
        SceneProfileDraft, SceneProfileSyncHostPort, SceneProfileSyncProgress,
        SceneProfileSyncTaskScope,
    },
};
use a4n_prompt::rendering::PromptRenderer;

pub(crate) struct SceneProfileRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> SceneProfileRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn scene_profile_sync_task_scope_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<SceneProfileSyncTaskScope, AppError> {
        self.orchestrator
            .scene_profile_sync_task_scope_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::scene::scene_profile::Model>, AppError> {
        self.orchestrator
            .scene_profiles_by_project(project_id)
            .await
    }

    pub(crate) async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        self.orchestrator
            .scene_profile_by_id(scene_profile_id)
            .await
    }

    pub(crate) async fn create_scene_profile_validated(
        &self,
        project_id: i64,
        draft: SceneProfileDraft,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        self.orchestrator
            .create_scene_profile_validated(project_id, draft)
            .await
    }

    pub(crate) async fn update_scene_profile_validated(
        &self,
        scene_profile_id: i64,
        draft: SceneProfileDraft,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        self.orchestrator
            .update_scene_profile_validated(scene_profile_id, draft)
            .await
    }

    pub(crate) async fn delete_scene_profile_records_validated(
        &self,
        scene_profile_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_scene_profile_records_validated(scene_profile_id)
            .await
    }

    pub(crate) async fn sync_scene_profiles_for_chapters_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: Option<i32>,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
        progress: Option<&SceneProfileSyncProgress<'_>>,
    ) -> Result<serde_json::Value, AppError> {
        self.orchestrator
            .sync_scene_profiles_for_chapters_for_user_validated(
                user_id,
                project_id,
                chapter_number,
                prompt_renderer,
                host_port,
                progress,
            )
            .await
    }

    pub(crate) async fn scene_occurrence_count(
        &self,
        project_id: i64,
        scene_profile_id: i64,
    ) -> Result<u64, AppError> {
        self.orchestrator
            .scene_occurrence_count(project_id, scene_profile_id)
            .await
    }

    pub(crate) async fn scene_occurrence_counts_by_profile(
        &self,
        project_id: i64,
        profile_ids: &[i64],
    ) -> Result<BTreeMap<i64, u64>, AppError> {
        self.orchestrator
            .scene_occurrence_counts_by_profile(project_id, profile_ids)
            .await
    }
}

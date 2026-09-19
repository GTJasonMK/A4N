use a4n_db::{DbHandle, scene_profile::SceneSyncRecordResult};
use a4n_domain::scene_profile::SceneProfileEvidenceRecordInput;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::scene_profile::{
    SceneSyncInput, normalize_scene_sync_record_input, scene_profile_db_error,
};

#[derive(Clone)]
pub(super) struct SceneProfileApplication {
    database: DbHandle,
}

impl SceneProfileApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::scene::scene_profile::Model>, AppError> {
        a4n_db::scene_profile::scene_profiles_by_project(self.database.connection(), project_id)
            .await
            .map_err(scene_profile_db_error)
    }

    async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        a4n_db::scene_profile::scene_profile_by_id(self.database.connection(), scene_profile_id)
            .await
            .map_err(scene_profile_db_error)
    }

    /// 规范化同步草稿后与 generation_run 一并替换章节场景同步结果。
    async fn replace_chapter_scene_sync_from_input_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        source: a4n_db::project::ChapterSourceSnapshot,
        input: SceneSyncInput,
        profile_evidence: Vec<SceneProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<SceneSyncRecordResult, AppError> {
        let sync_input = normalize_scene_sync_record_input(input)?;
        a4n_db::scene_profile::replace_chapter_scene_sync_with_generation_run_records(
            self.database.connection(),
            project_id,
            chapter_id,
            chapter_number,
            source,
            sync_input,
            profile_evidence,
            generation_run,
        )
        .await
        .map_err(scene_profile_db_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db_models::models::scene::scene_profile::Model>, AppError> {
        self.scene_profile
            .scene_profiles_by_project(project_id)
            .await
    }

    pub(crate) async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_db_models::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .scene_profile_by_id(scene_profile_id)
            .await
    }

    /// 场景同步落库：规范化业务输入后与 generation_run 一并写入。
    pub(crate) async fn replace_chapter_scene_sync_from_input_validated(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        source: a4n_db::project::ChapterSourceSnapshot,
        input: SceneSyncInput,
        profile_evidence: Vec<SceneProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<SceneSyncRecordResult, AppError> {
        self.scene_profile
            .replace_chapter_scene_sync_from_input_validated(
                project_id,
                chapter_id,
                chapter_number,
                source,
                input,
                profile_evidence,
                generation_run,
            )
            .await
    }
}

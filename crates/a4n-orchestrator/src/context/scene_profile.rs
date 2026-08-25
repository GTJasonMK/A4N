use std::collections::BTreeMap;

use a4n_api_contract::profile::SceneProfileInput;
use a4n_core::error::AppError;
use a4n_db::DbHandle;

use super::OrchestratorContext;
use crate::scene_profile::{
    SceneProfileEvidenceRecordInput, SceneSyncInput, SceneSyncRecordResult,
    normalize_scene_profile_record_input, normalize_scene_sync_record_input,
    scene_profile_db_error,
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
    ) -> Result<Vec<a4n_core::models::scene::scene_profile::Model>, AppError> {
        a4n_db::scene_profile::scene_profiles_by_project(self.database.connection(), project_id)
            .await
            .map_err(scene_profile_db_error)
    }

    async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        a4n_db::scene_profile::scene_profile_by_id(self.database.connection(), scene_profile_id)
            .await
            .map_err(scene_profile_db_error)
    }

    async fn delete_scene_profile_records(&self, scene_profile_id: i64) -> Result<(), AppError> {
        a4n_db::scene_profile::delete_scene_profile_records(
            self.database.connection(),
            scene_profile_id,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    /// 规范化 API 输入后创建场景档案。
    async fn create_scene_profile_validated(
        &self,
        project_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        let record = normalize_scene_profile_record_input(input)?;
        a4n_db::scene_profile::create_scene_profile_record_checked(
            self.database.connection(),
            project_id,
            record,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    /// 规范化 API 输入后更新场景档案。
    async fn update_scene_profile_validated(
        &self,
        scene_profile_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        let record = normalize_scene_profile_record_input(input)?;
        a4n_db::scene_profile::update_scene_profile_record_checked(
            self.database.connection(),
            scene_profile_id,
            record,
        )
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

    async fn scene_occurrence_count(
        &self,
        project_id: i64,
        scene_profile_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::scene_profile::scene_occurrence_count(
            self.database.connection(),
            project_id,
            scene_profile_id,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    async fn scene_occurrence_counts_by_profile(
        &self,
        project_id: i64,
        profile_ids: &[i64],
    ) -> Result<BTreeMap<i64, u64>, AppError> {
        a4n_db::scene_profile::scene_occurrence_counts_by_profile(
            self.database.connection(),
            project_id,
            profile_ids,
        )
        .await
        .map_err(scene_profile_db_error)
    }
}

impl OrchestratorContext {
    pub async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::scene::scene_profile::Model>, AppError> {
        self.scene_profile
            .scene_profiles_by_project(project_id)
            .await
    }

    pub async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .scene_profile_by_id(scene_profile_id)
            .await
    }

    /// 创建场景档案：规范化 API 输入后落库。
    pub async fn create_scene_profile_validated(
        &self,
        project_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .create_scene_profile_validated(project_id, input)
            .await
    }

    /// 更新场景档案：规范化 API 输入后落库。
    pub async fn update_scene_profile_validated(
        &self,
        scene_profile_id: i64,
        input: SceneProfileInput,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        self.scene_profile
            .update_scene_profile_validated(scene_profile_id, input)
            .await
    }

    /// 删除场景档案记录（下游图片清理仍由 Host 执行）。
    pub async fn delete_scene_profile_records_validated(
        &self,
        scene_profile_id: i64,
    ) -> Result<(), AppError> {
        if scene_profile_id <= 0 {
            return Err(AppError::InvalidArgument(
                "场景档案 ID 必须大于 0".to_string(),
            ));
        }
        self.scene_profile
            .delete_scene_profile_records(scene_profile_id)
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

    pub async fn scene_occurrence_count(
        &self,
        project_id: i64,
        scene_profile_id: i64,
    ) -> Result<u64, AppError> {
        self.scene_profile
            .scene_occurrence_count(project_id, scene_profile_id)
            .await
    }

    pub async fn scene_occurrence_counts_by_profile(
        &self,
        project_id: i64,
        profile_ids: &[i64],
    ) -> Result<BTreeMap<i64, u64>, AppError> {
        self.scene_profile
            .scene_occurrence_counts_by_profile(project_id, profile_ids)
            .await
    }
}

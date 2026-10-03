use a4n_api_contract::profile::ObjectProfileInput;
use a4n_db::{DbHandle, asset::ObjectProfileUpsertRecordResult};
use a4n_db_models::models::asset::object_profile;
use a4n_domain::{
    chapter::ChapterSourceSnapshot, generation::RecordCompletedGenerationRunInput,
    object_profile::ObjectProfileEvidenceRecordInput,
};
use a4n_error::AppError;

use super::OrchestratorContext;

use crate::asset::{asset_error, object_profiles::normalize_object_profile_record_inputs};

#[derive(Clone)]
pub(super) struct AssetApplication {
    database: DbHandle,
}

impl AssetApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn object_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        a4n_db::asset::object_profiles_by_project(self.database.connection(), project_id)
            .await
            .map_err(asset_error)
    }

    async fn object_profile_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_domain::object_profile::ObjectProfileRecord>, AppError> {
        a4n_db::asset::object_profile_records_by_project(self.database.connection(), project_id)
            .await
            .map_err(asset_error)
    }

    async fn object_profile_by_id(
        &self,
        object_profile_id: i64,
    ) -> Result<object_profile::Model, AppError> {
        a4n_db::asset::object_profile_by_id(self.database.connection(), object_profile_id)
            .await
            .map_err(asset_error)
    }

    /// 批量规范化草稿后与 generation_run 一并 upsert。
    async fn upsert_object_profiles_from_drafts_validated(
        &self,
        project_id: i64,
        drafts: Vec<ObjectProfileInput>,
        source_chapters: Vec<ChapterSourceSnapshot>,
        profile_evidence: Vec<ObjectProfileEvidenceRecordInput>,
        generation_run: RecordCompletedGenerationRunInput,
    ) -> Result<ObjectProfileUpsertRecordResult, AppError> {
        let inputs = normalize_object_profile_record_inputs(drafts)?;
        a4n_db::asset::upsert_object_profiles_with_generation_run_record(
            self.database.connection(),
            project_id,
            inputs,
            source_chapters,
            profile_evidence,
            generation_run,
        )
        .await
        .map_err(asset_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn object_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        self.asset.object_profiles_by_project(project_id).await
    }

    pub(crate) async fn object_profile_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_domain::object_profile::ObjectProfileRecord>, AppError> {
        self.asset
            .object_profile_records_by_project(project_id)
            .await
    }

    pub(crate) async fn object_profile_by_id(
        &self,
        object_profile_id: i64,
    ) -> Result<object_profile::Model, AppError> {
        self.asset.object_profile_by_id(object_profile_id).await
    }

    /// 发现路径：批量规范化草稿后与 generation_run 一并 upsert。
    pub(crate) async fn upsert_object_profiles_from_drafts_validated(
        &self,
        project_id: i64,
        drafts: Vec<ObjectProfileInput>,
        source_chapters: Vec<ChapterSourceSnapshot>,
        profile_evidence: Vec<ObjectProfileEvidenceRecordInput>,
        generation_run: RecordCompletedGenerationRunInput,
    ) -> Result<ObjectProfileUpsertRecordResult, AppError> {
        self.asset
            .upsert_object_profiles_from_drafts_validated(
                project_id,
                drafts,
                source_chapters,
                profile_evidence,
                generation_run,
            )
            .await
    }
}

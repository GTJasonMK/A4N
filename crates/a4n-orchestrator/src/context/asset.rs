use a4n_core::error::AppError;
use a4n_core::models::asset::object_profile;
use a4n_db::DbHandle;

use super::OrchestratorContext;

use crate::asset::{
    ObjectProfileDraft, ObjectProfileEvidenceRecordInput, ObjectProfileUpsertRecordResult,
    asset_error, normalize_object_profile_record_input, normalize_object_profile_record_inputs,
};

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

    async fn object_profile_by_id(
        &self,
        object_profile_id: i64,
    ) -> Result<object_profile::Model, AppError> {
        a4n_db::asset::object_profile_by_id(self.database.connection(), object_profile_id)
            .await
            .map_err(asset_error)
    }

    async fn delete_object_profile_record(&self, object_profile_id: i64) -> Result<(), AppError> {
        a4n_db::asset::delete_object_profile_record(self.database.connection(), object_profile_id)
            .await
            .map_err(asset_error)
    }

    /// 规范化草稿后创建物体档案。
    async fn create_object_profile_validated(
        &self,
        project_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        let record = normalize_object_profile_record_input(draft)?;
        a4n_db::asset::create_object_profile_record(self.database.connection(), project_id, record)
            .await
            .map_err(asset_error)
    }

    /// 规范化草稿后更新物体档案。
    async fn update_object_profile_validated(
        &self,
        object_profile_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        let record = normalize_object_profile_record_input(draft)?;
        a4n_db::asset::update_object_profile_record(
            self.database.connection(),
            object_profile_id,
            record,
        )
        .await
        .map_err(asset_error)
    }

    /// 批量规范化草稿后与 generation_run 一并 upsert。
    async fn upsert_object_profiles_from_drafts_validated(
        &self,
        project_id: i64,
        drafts: Vec<ObjectProfileDraft>,
        source_chapters: Vec<a4n_db::project::ChapterSourceSnapshot>,
        profile_evidence: Vec<ObjectProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
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
    pub async fn object_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        self.asset.object_profiles_by_project(project_id).await
    }

    pub async fn object_profile_by_id(
        &self,
        object_profile_id: i64,
    ) -> Result<object_profile::Model, AppError> {
        self.asset.object_profile_by_id(object_profile_id).await
    }

    /// 创建物体档案：规范化草稿后落库。
    pub async fn create_object_profile_validated(
        &self,
        project_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        self.asset
            .create_object_profile_validated(project_id, draft)
            .await
    }

    /// 更新物体档案：规范化草稿后落库。
    pub async fn update_object_profile_validated(
        &self,
        object_profile_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        self.asset
            .update_object_profile_validated(object_profile_id, draft)
            .await
    }

    /// 发现路径：批量规范化草稿后与 generation_run 一并 upsert。
    pub(crate) async fn upsert_object_profiles_from_drafts_validated(
        &self,
        project_id: i64,
        drafts: Vec<ObjectProfileDraft>,
        source_chapters: Vec<a4n_db::project::ChapterSourceSnapshot>,
        profile_evidence: Vec<ObjectProfileEvidenceRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
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

    /// 删除物体档案记录（下游图片清理仍由 Host 执行）。
    pub async fn delete_object_profile_record_validated(
        &self,
        object_profile_id: i64,
    ) -> Result<(), AppError> {
        if object_profile_id <= 0 {
            return Err(AppError::InvalidArgument(
                "物体档案 ID 必须大于 0".to_string(),
            ));
        }
        self.asset
            .delete_object_profile_record(object_profile_id)
            .await
    }
}

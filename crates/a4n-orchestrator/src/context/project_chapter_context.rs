use a4n_core::{
    error::AppError,
    models::{
        asset::object_profile,
        character::{character_state_index, foreshadowing_index, protagonist_profile},
        scene::scene_profile,
    },
};
use a4n_db::{
    DbHandle,
    project::{
        ChapterGenerationReferenceContextRows, ForeshadowingAnalysisDraft,
        ForeshadowingAnalysisSyncResult,
    },
};

use super::OrchestratorContext;
use crate::project::{
    foreshadowing_db_error_to_app_error, project_chapter_context_error_to_app_error,
};

#[derive(Clone)]
pub(super) struct ProjectChapterContextApplication {
    database: DbHandle,
}

impl ProjectChapterContextApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn generation_reference_rows(
        &self,
        project_id: i64,
    ) -> Result<ChapterGenerationReferenceContextRows, AppError> {
        a4n_db::project::chapter_generation_reference_context_rows(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn sync_analysis_foreshadowing(
        &self,
        source: a4n_db::project::ChapterSourceSnapshot,
        drafts: Vec<ForeshadowingAnalysisDraft>,
        resolved_ids: Vec<i64>,
    ) -> Result<ForeshadowingAnalysisSyncResult, AppError> {
        a4n_db::project::sync_chapter_analysis_foreshadowing(
            self.database.connection(),
            source,
            drafts,
            resolved_ids,
        )
        .await
        .map_err(foreshadowing_db_error_to_app_error)
    }

    async fn foreshadowing_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::project::foreshadowing_records_by_project(self.database.connection(), project_id)
            .await
            .map_err(foreshadowing_db_error_to_app_error)
    }

    async fn create_foreshadowing_record(
        &self,
        project_id: i64,
        input: a4n_db::project::ForeshadowingRecordInput,
    ) -> Result<foreshadowing_index::Model, AppError> {
        a4n_db::project::create_foreshadowing_record(self.database.connection(), project_id, input)
            .await
            .map_err(foreshadowing_db_error_to_app_error)
    }

    async fn update_foreshadowing_record(
        &self,
        project_id: i64,
        foreshadowing_id: i64,
        expected_updated_at: &str,
        input: a4n_db::project::ForeshadowingRecordInput,
    ) -> Result<foreshadowing_index::Model, AppError> {
        a4n_db::project::update_foreshadowing_record(
            self.database.connection(),
            project_id,
            foreshadowing_id,
            expected_updated_at,
            input,
        )
        .await
        .map_err(foreshadowing_db_error_to_app_error)
    }

    async fn delete_foreshadowing_record(
        &self,
        project_id: i64,
        foreshadowing_id: i64,
        expected_updated_at: &str,
    ) -> Result<foreshadowing_index::Model, AppError> {
        a4n_db::project::delete_foreshadowing_record(
            self.database.connection(),
            project_id,
            foreshadowing_id,
            expected_updated_at,
        )
        .await
        .map_err(foreshadowing_db_error_to_app_error)
    }

    async fn object_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        a4n_db::project::object_profiles_before_chapter(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn scene_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<scene_profile::Model>, AppError> {
        a4n_db::project::scene_profiles_before_chapter(
            self.database.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn protagonist_profiles_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        a4n_db::project::protagonist_profiles_for_project(self.database.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    async fn character_states_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        a4n_db::project::character_states_for_profiles_before_chapter(
            self.database.connection(),
            profile_ids,
            chapter_number,
            limit,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn snapshot_profile_ids_for_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::project::snapshot_profile_ids_for_chapter(
            self.database.connection(),
            profile_ids,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub async fn foreshadowing_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        self.project_chapter_context
            .foreshadowing_records_by_project(project_id)
            .await
    }

    pub async fn create_foreshadowing_record_validated(
        &self,
        project_id: i64,
        input: a4n_db::project::ForeshadowingRecordInput,
    ) -> Result<foreshadowing_index::Model, AppError> {
        self.project_chapter_context
            .create_foreshadowing_record(project_id, input)
            .await
    }

    pub async fn update_foreshadowing_record_validated(
        &self,
        project_id: i64,
        foreshadowing_id: i64,
        expected_updated_at: &str,
        input: a4n_db::project::ForeshadowingRecordInput,
    ) -> Result<foreshadowing_index::Model, AppError> {
        self.project_chapter_context
            .update_foreshadowing_record(project_id, foreshadowing_id, expected_updated_at, input)
            .await
    }

    pub async fn delete_foreshadowing_record_validated(
        &self,
        project_id: i64,
        foreshadowing_id: i64,
        expected_updated_at: &str,
    ) -> Result<foreshadowing_index::Model, AppError> {
        self.project_chapter_context
            .delete_foreshadowing_record(project_id, foreshadowing_id, expected_updated_at)
            .await
    }

    pub(crate) async fn chapter_generation_reference_context_rows(
        &self,
        project_id: i64,
    ) -> Result<ChapterGenerationReferenceContextRows, AppError> {
        self.project_chapter_context
            .generation_reference_rows(project_id)
            .await
    }

    pub(crate) async fn sync_chapter_analysis_foreshadowing(
        &self,
        source: a4n_db::project::ChapterSourceSnapshot,
        drafts: Vec<ForeshadowingAnalysisDraft>,
        resolved_ids: Vec<i64>,
    ) -> Result<ForeshadowingAnalysisSyncResult, AppError> {
        self.project_chapter_context
            .sync_analysis_foreshadowing(source, drafts, resolved_ids)
            .await
    }

    pub(crate) async fn object_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        self.project_chapter_context
            .object_profiles_before_chapter(project_id, chapter_number)
            .await
    }

    pub(crate) async fn scene_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<scene_profile::Model>, AppError> {
        self.project_chapter_context
            .scene_profiles_before_chapter(project_id, chapter_number)
            .await
    }

    pub async fn protagonist_profiles_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.project_chapter_context
            .protagonist_profiles_for_project(project_id)
            .await
    }

    pub(crate) async fn character_states_for_profiles_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        self.project_chapter_context
            .character_states_before_chapter(profile_ids, chapter_number, limit)
            .await
    }

    pub async fn snapshot_profile_ids_for_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<i64>, AppError> {
        self.project_chapter_context
            .snapshot_profile_ids_for_chapter(profile_ids, chapter_number)
            .await
    }
}

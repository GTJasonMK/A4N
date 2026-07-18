use a4n_db::project::{
    ChapterGenerationReferenceContextRows, ForeshadowingAnalysisDraft,
    ForeshadowingAnalysisSyncResult,
};

use super::super::{
    character_state_index, foreshadowing_db_error_to_app_error, object_profile,
    project_chapter_context_error_to_app_error, protagonist_profile, scene_profile,
};
use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn chapter_generation_reference_context_rows(
        &self,
        project_id: i64,
    ) -> Result<ChapterGenerationReferenceContextRows, AppError> {
        a4n_db::project::chapter_generation_reference_context_rows(self.db.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn sync_chapter_analysis_foreshadowing(
        &self,
        project_id: i64,
        chapter_number: i32,
        drafts: Vec<ForeshadowingAnalysisDraft>,
    ) -> Result<ForeshadowingAnalysisSyncResult, AppError> {
        a4n_db::project::sync_chapter_analysis_foreshadowing(
            self.db.connection(),
            project_id,
            chapter_number,
            drafts,
        )
        .await
        .map_err(foreshadowing_db_error_to_app_error)
    }

    pub async fn object_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        a4n_db::project::object_profiles_before_chapter(
            self.db.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn scene_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<scene_profile::Model>, AppError> {
        a4n_db::project::scene_profiles_before_chapter(
            self.db.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn protagonist_profiles_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        a4n_db::project::protagonist_profiles_for_project(self.db.connection(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn character_states_for_profiles_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        a4n_db::project::character_states_for_profiles_before_chapter(
            self.db.connection(),
            profile_ids,
            chapter_number,
            limit,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    pub async fn snapshot_profile_ids_for_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<i64>, AppError> {
        a4n_db::project::snapshot_profile_ids_for_chapter(
            self.db.connection(),
            profile_ids,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }
}

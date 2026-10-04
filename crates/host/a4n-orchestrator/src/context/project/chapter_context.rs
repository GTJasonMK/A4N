use a4n_db::{
    DbHandle,
    project::{
        ChapterGenerationReferenceContextRows, ChapterObjectProfileContext,
        ChapterSceneProfileContext, ForeshadowingAnalysisDraft, ForeshadowingAnalysisSyncResult,
    },
};
use a4n_domain::chapter::ChapterSourceSnapshot;
use a4n_domain::character::{CharacterProfileRecord, CharacterStateIndexRecord};
use a4n_error::AppError;

use super::super::OrchestratorContext;
use crate::project::{
    foreshadowing_db_error_to_app_error, project_chapter_context_error_to_app_error,
};

#[derive(Clone)]
pub(in crate::context) struct ProjectChapterContextApplication {
    database: DbHandle,
}

impl ProjectChapterContextApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn generation_reference_rows(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<ChapterGenerationReferenceContextRows, AppError> {
        a4n_db::project::chapter_generation_reference_context_rows(
            self.database.executor(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn sync_analysis_foreshadowing(
        &self,
        source: ChapterSourceSnapshot,
        drafts: Vec<ForeshadowingAnalysisDraft>,
        updates: Vec<a4n_db::project::ForeshadowingFactUpdate>,
    ) -> Result<ForeshadowingAnalysisSyncResult, AppError> {
        a4n_db::project::sync_chapter_analysis_foreshadowing(
            self.database.executor(),
            source,
            drafts,
            updates,
        )
        .await
        .map_err(foreshadowing_db_error_to_app_error)
    }

    async fn object_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<ChapterObjectProfileContext>, AppError> {
        a4n_db::project::object_profiles_before_chapter(
            self.database.executor(),
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
    ) -> Result<Vec<ChapterSceneProfileContext>, AppError> {
        a4n_db::project::scene_profiles_before_chapter(
            self.database.executor(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }

    async fn protagonist_profiles_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileRecord>, AppError> {
        a4n_db::project::protagonist_profiles_for_project(self.database.executor(), project_id)
            .await
            .map_err(project_chapter_context_error_to_app_error)
    }

    async fn protagonist_profile_records_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_domain::character::CharacterProfileRecord>, AppError> {
        a4n_db::character::profile_records_by_project(self.database.executor(), project_id)
            .await
            .map_err(AppError::from)
    }

    async fn character_states_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<CharacterStateIndexRecord>, AppError> {
        a4n_db::project::character_states_for_profiles_before_chapter(
            self.database.executor(),
            profile_ids,
            chapter_number,
            limit,
        )
        .await
        .map_err(project_chapter_context_error_to_app_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn chapter_generation_reference_context_rows(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<ChapterGenerationReferenceContextRows, AppError> {
        self.project_chapter_context
            .generation_reference_rows(project_id, chapter_number)
            .await
    }

    pub(crate) async fn sync_chapter_analysis_foreshadowing(
        &self,
        source: ChapterSourceSnapshot,
        drafts: Vec<ForeshadowingAnalysisDraft>,
        updates: Vec<a4n_db::project::ForeshadowingFactUpdate>,
    ) -> Result<ForeshadowingAnalysisSyncResult, AppError> {
        self.project_chapter_context
            .sync_analysis_foreshadowing(source, drafts, updates)
            .await
    }

    pub(crate) async fn object_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<ChapterObjectProfileContext>, AppError> {
        self.project_chapter_context
            .object_profiles_before_chapter(project_id, chapter_number)
            .await
    }

    pub(crate) async fn scene_profiles_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Vec<ChapterSceneProfileContext>, AppError> {
        self.project_chapter_context
            .scene_profiles_before_chapter(project_id, chapter_number)
            .await
    }

    pub(crate) async fn protagonist_profiles_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<CharacterProfileRecord>, AppError> {
        self.project_chapter_context
            .protagonist_profiles_for_project(project_id)
            .await
    }

    pub(crate) async fn protagonist_profile_records_for_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_domain::character::CharacterProfileRecord>, AppError> {
        self.project_chapter_context
            .protagonist_profile_records_for_project(project_id)
            .await
    }

    pub(crate) async fn character_states_for_profiles_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<CharacterStateIndexRecord>, AppError> {
        self.project_chapter_context
            .character_states_before_chapter(profile_ids, chapter_number, limit)
            .await
    }
}

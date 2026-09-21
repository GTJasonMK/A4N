use a4n_api_contract::profile::{
    CreateForeshadowingInput, DeleteForeshadowingInput, ForeshadowingWriteResult,
    UpdateForeshadowingInput,
};
use a4n_db::{
    DbHandle,
    project::{
        ChapterGenerationReferenceContextRows, ForeshadowingAnalysisDraft,
        ForeshadowingAnalysisSyncResult,
    },
};
use a4n_db_models::models::{
    asset::object_profile,
    character::{character_state_index, foreshadowing_index, protagonist_profile},
    scene::scene_profile,
};
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::project::{
    foreshadowing_db_error_to_app_error, project_chapter_context_error_to_app_error,
};
use a4n_domain::collaboration::ProjectAccessRole;

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

    async fn create_foreshadowing_record(
        &self,
        project_id: i64,
        input: a4n_domain::character::ForeshadowingRecordInput,
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
        input: a4n_domain::character::ForeshadowingRecordInput,
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
    pub(crate) async fn create_foreshadowing_record(
        &self,
        project_id: i64,
        input: a4n_domain::character::ForeshadowingRecordInput,
    ) -> Result<foreshadowing_index::Model, AppError> {
        self.project_chapter_context
            .create_foreshadowing_record(project_id, input)
            .await
    }

    pub(crate) async fn update_foreshadowing_record(
        &self,
        project_id: i64,
        foreshadowing_id: i64,
        expected_updated_at: &str,
        input: a4n_domain::character::ForeshadowingRecordInput,
    ) -> Result<foreshadowing_index::Model, AppError> {
        self.project_chapter_context
            .update_foreshadowing_record(project_id, foreshadowing_id, expected_updated_at, input)
            .await
    }

    pub(crate) async fn delete_foreshadowing_record(
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

    pub(crate) async fn protagonist_profiles_for_project(
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

    pub(crate) async fn snapshot_profile_ids_for_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<i64>, AppError> {
        self.project_chapter_context
            .snapshot_profile_ids_for_chapter(profile_ids, chapter_number)
            .await
    }
}

async fn foreshadowing_rag_warning(
    context: &OrchestratorContext,
    record: &foreshadowing_index::Model,
) -> Vec<String> {
    match context
        .sync_foreshadowing_rag(record.project_id, record)
        .await
    {
        Ok(()) => Vec::new(),
        Err(error) => {
            tracing::warn!(
                project_id = record.project_id,
                foreshadowing_id = record.id,
                error = %error,
                "伏笔已保存，但 RAG 索引失败"
            );
            vec!["伏笔已保存，但检索索引更新失败，可稍后重试".to_string()]
        }
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::profile::ForeshadowingMutationApplicationPort for OrchestratorContext {
    async fn create(
        &self,
        user_id: i64,
        input: CreateForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let record = self
            .create_foreshadowing_record(input.project_id, input.record)
            .await?;
        Ok(ForeshadowingWriteResult {
            warnings: foreshadowing_rag_warning(self, &record).await,
        })
    }

    async fn update(
        &self,
        user_id: i64,
        input: UpdateForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let record = self
            .update_foreshadowing_record(
                input.project_id,
                input.foreshadowing_id,
                &input.expected_updated_at,
                input.record,
            )
            .await?;
        Ok(ForeshadowingWriteResult {
            warnings: foreshadowing_rag_warning(self, &record).await,
        })
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteForeshadowingInput,
    ) -> Result<ForeshadowingWriteResult, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let project_id = input.project_id;
        let deleted = self
            .delete_foreshadowing_record(
                project_id,
                input.foreshadowing_id,
                &input.expected_updated_at,
            )
            .await?;
        let warnings = match self
            .delete_foreshadowing_rag_by_ids(project_id, &[deleted.id])
            .await
        {
            Ok(_) => Vec::new(),
            Err(error) => {
                tracing::warn!(
                    project_id,
                    foreshadowing_id = deleted.id,
                    error = %error,
                    "伏笔已删除，但 RAG 清理失败"
                );
                vec!["伏笔已删除，但检索索引清理失败，可稍后重试".to_string()]
            }
        };
        Ok(ForeshadowingWriteResult { warnings })
    }
}

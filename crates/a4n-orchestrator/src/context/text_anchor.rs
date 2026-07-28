//! 文本锚点领域私有 owner。
//!
//! 持有 `DbHandle`，封装章节正文锚点 ensure/read 与档案证据 merge。

use a4n_core::error::AppError;
use a4n_core::models::project::{profile_evidence, text_anchor};
use a4n_db::DbHandle;
use a4n_db::text_anchor::{
    CurrentChapterContentAnchorRows, TextAnchorDbError, TextAnchorEvidenceInput,
    TextAnchorProfileKind,
};

use super::OrchestratorContext;

#[derive(Clone)]
pub(super) struct TextAnchorApplication {
    database: DbHandle,
}

impl TextAnchorApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn ensure_chapter_content_anchor_records(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        a4n_db::text_anchor::ensure_chapter_content_anchor_records(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(text_anchor_db_error)
    }

    async fn read_current_chapter_content_anchor_records(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<CurrentChapterContentAnchorRows, AppError> {
        a4n_db::text_anchor::read_current_chapter_content_anchor_records(
            self.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(text_anchor_db_error)
    }

    async fn merge_profile_evidence_records(
        &self,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        profile_kind: TextAnchorProfileKind,
        profile_id: i64,
        refs: Vec<TextAnchorEvidenceInput>,
    ) -> Result<Vec<profile_evidence::Model>, AppError> {
        a4n_db::text_anchor::merge_profile_evidence_records(
            self.connection(),
            root_project_id,
            source_snapshot_id,
            profile_kind,
            profile_id,
            refs,
        )
        .await
        .map_err(text_anchor_db_error)
    }
}

fn text_anchor_db_error(error: TextAnchorDbError) -> AppError {
    match error {
        TextAnchorDbError::Database(error) => AppError::Database(error),
        TextAnchorDbError::NotFound(message) => AppError::NotFound(message),
        TextAnchorDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
    }
}

impl OrchestratorContext {
    pub async fn ensure_chapter_content_anchor_records(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<text_anchor::Model>, AppError> {
        self.text_anchor
            .ensure_chapter_content_anchor_records(project_id, chapter_id)
            .await
    }

    pub async fn read_current_chapter_content_anchor_records(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<CurrentChapterContentAnchorRows, AppError> {
        self.text_anchor
            .read_current_chapter_content_anchor_records(project_id, chapter_id)
            .await
    }

    pub async fn merge_profile_evidence_records(
        &self,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        profile_kind: TextAnchorProfileKind,
        profile_id: i64,
        refs: Vec<TextAnchorEvidenceInput>,
    ) -> Result<Vec<profile_evidence::Model>, AppError> {
        self.text_anchor
            .merge_profile_evidence_records(
                root_project_id,
                source_snapshot_id,
                profile_kind,
                profile_id,
                refs,
            )
            .await
    }
}

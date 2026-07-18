pub use a4n_db::text_anchor::{
    CurrentChapterContentAnchorRows, EVIDENCE_ROLE_APPEARANCE, EVIDENCE_ROLE_BEHAVIOR,
    EVIDENCE_ROLE_FIRST_SEEN, EVIDENCE_ROLE_LOCATION, EVIDENCE_ROLE_RELATIONSHIP,
    EVIDENCE_ROLE_STATE_CHANGE, SOURCE_KIND_CHAPTER_CONTENT, TextAnchorEvidenceInput,
    TextAnchorProfileKind,
};

use crate::OrchestratorContext;
use a4n_core::error::AppError;
use a4n_db::text_anchor::TextAnchorDbError;

impl OrchestratorContext {
    pub async fn ensure_chapter_content_anchor_records(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<a4n_core::models::project::text_anchor::Model>, AppError> {
        a4n_db::text_anchor::ensure_chapter_content_anchor_records(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(text_anchor_db_error)
    }

    pub async fn read_current_chapter_content_anchor_records(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<CurrentChapterContentAnchorRows, AppError> {
        a4n_db::text_anchor::read_current_chapter_content_anchor_records(
            self.db.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(text_anchor_db_error)
    }

    pub async fn merge_profile_evidence_records(
        &self,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        profile_kind: TextAnchorProfileKind,
        profile_id: i64,
        refs: Vec<TextAnchorEvidenceInput>,
    ) -> Result<Vec<a4n_core::models::project::profile_evidence::Model>, AppError> {
        a4n_db::text_anchor::merge_profile_evidence_records(
            self.db.connection(),
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

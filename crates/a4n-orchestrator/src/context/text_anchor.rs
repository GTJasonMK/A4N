//! 文本锚点领域私有 owner。
//!
//! 持有 `DbHandle`，封装章节正文锚点 ensure/read 与档案证据章节查询。

use a4n_core::error::AppError;
use a4n_core::models::project::text_anchor;
use a4n_db::DbHandle;
use a4n_db::text_anchor::{CurrentChapterContentAnchorRows, TextAnchorProfileKind};

use super::OrchestratorContext;
use crate::text_anchor::text_anchor_db_error;

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

    async fn ensure_chapter_content_anchor_records_batch(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<Vec<text_anchor::Model>>, AppError> {
        a4n_db::text_anchor::ensure_chapter_content_anchor_records_batch(
            self.connection(),
            project_id,
            chapter_ids,
        )
        .await
        .map_err(text_anchor_db_error)
    }

    async fn read_current_chapter_content_anchor_records_batch(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<CurrentChapterContentAnchorRows>, AppError> {
        a4n_db::text_anchor::read_current_chapter_content_anchor_records_batch(
            self.connection(),
            project_id,
            chapter_ids,
        )
        .await
        .map_err(text_anchor_db_error)
    }

    async fn earliest_current_profile_evidence_chapter(
        &self,
        root_project_id: i64,
        profile_kind: TextAnchorProfileKind,
        profile_id: i64,
    ) -> Result<Option<i32>, AppError> {
        a4n_db::text_anchor::earliest_current_profile_evidence_chapter(
            self.connection(),
            root_project_id,
            profile_kind,
            profile_id,
        )
        .await
        .map_err(text_anchor_db_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn ensure_chapter_content_anchor_records_batch(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<Vec<text_anchor::Model>>, AppError> {
        self.text_anchor
            .ensure_chapter_content_anchor_records_batch(project_id, chapter_ids)
            .await
    }

    pub(crate) async fn require_current_chapter_content_anchors_for_preview_batch(
        &self,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<Vec<text_anchor::Model>>, AppError> {
        let rows = self
            .text_anchor
            .read_current_chapter_content_anchor_records_batch(project_id, chapter_ids)
            .await?;
        rows.into_iter()
            .map(|row| {
                if row.anchors.is_empty() {
                    return Err(AppError::InvalidState {
                        current: format!(
                            "第 {} 章缺少当前正文文本锚点",
                            row.chapter.chapter_number
                        ),
                        operation: "预览模型请求".to_string(),
                    });
                }
                if row
                    .anchors
                    .iter()
                    .any(|anchor| anchor.source_version_hash != row.source_version_hash)
                {
                    return Err(AppError::InvalidState {
                        current: format!("第 {} 章正文文本锚点已过期", row.chapter.chapter_number),
                        operation: "预览模型请求".to_string(),
                    });
                }
                Ok(row.anchors)
            })
            .collect()
    }

    pub(crate) async fn earliest_current_profile_evidence_chapter(
        &self,
        root_project_id: i64,
        profile_kind: TextAnchorProfileKind,
        profile_id: i64,
    ) -> Result<Option<i32>, AppError> {
        self.text_anchor
            .earliest_current_profile_evidence_chapter(root_project_id, profile_kind, profile_id)
            .await
    }
}

mod analysis;
mod edit;
mod generation;
mod import;
mod queries;
mod types;

use a4n_db_models::models::chapter::chapter;
use chrono::NaiveDateTime;
use sea_orm::Set;

pub use analysis::{
    chapter_source_snapshots_match_for_update, lock_chapter_analysis_source,
    store_chapter_analysis_record, update_chapter_style_tags_record,
};
pub use edit::save_chapter_content_edit_record;
pub use generation::{
    begin_chapter_generation_record, finish_generated_chapter_content_record_in_transaction,
    restore_chapter_generation_status_record,
};
pub use import::import_chapter_content_record;
pub use queries::{chapter_by_id_for_analysis, chapters_for_project};
pub use types::{
    ChapterContentEditError, ChapterContentImportError, ChapterGenerationExistingRecord,
    ChapterGenerationLifecycleError, GeneratedChapterContentRecordInput,
};

pub(crate) fn assign_chapter_content(
    active: &mut chapter::ActiveModel,
    content: String,
    word_count: i32,
    status: &str,
    summary: Option<String>,
    updated_at: NaiveDateTime,
) {
    active.content = Set(content);
    active.word_count = Set(word_count);
    active.status = Set(status.to_string());
    active.summary = Set(summary);
    active.style_tags = Set(None);
    active.analysis_data = Set(None);
    active.updated_at = Set(updated_at);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assigning_chapter_content_invalidates_old_analysis() {
        let now = chrono::Utc::now().naive_utc();
        let mut active = chapter::ActiveModel {
            summary: Set(Some("旧摘要".to_string())),
            style_tags: Set(Some(serde_json::json!(["旧风格"]))),
            analysis_data: Set(Some(serde_json::json!({"old": true}))),
            ..Default::default()
        };

        assign_chapter_content(
            &mut active,
            "新正文".to_string(),
            3,
            chapter::STATUS_DRAFT,
            None,
            now,
        );

        assert_eq!(active.content, Set("新正文".to_string()));
        assert_eq!(active.word_count, Set(3));
        assert_eq!(active.status, Set(chapter::STATUS_DRAFT.to_string()));
        assert_eq!(active.summary, Set(None));
        assert_eq!(active.style_tags, Set(None));
        assert_eq!(active.analysis_data, Set(None));
        assert_eq!(active.updated_at, Set(now));
    }
}

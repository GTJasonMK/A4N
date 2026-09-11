mod trace;

use a4n_db::chapter_content::GeneratedChapterContentRecordInput;
use a4n_db_models::models::chapter::{chapter, chapter_outline};
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use chrono::Utc;
use sea_orm::DatabaseTransaction;

use super::super::ChapterGenerationService;
use super::chapter_generation_lifecycle_error_to_app_error;
use super::session::ChapterGenerationSession;

#[derive(Clone)]
pub(in crate::chapter_generation) struct ChapterGenerationTrace {
    pub(in crate::chapter_generation) request: ChatRequest,
    pub(in crate::chapter_generation) finish_reason: Option<String>,
    pub(in crate::chapter_generation) model_name: String,
}

impl ChapterGenerationService {
    pub(in crate::chapter_generation) async fn finish_generated_chapter_in_transaction(
        &self,
        session: &ChapterGenerationSession,
        content: &str,
        source_outline: Option<chapter_outline::Model>,
        chapter_outline: &str,
        generation_trace: Option<ChapterGenerationTrace>,
        db: &DatabaseTransaction,
    ) -> Result<chapter::Model, AppError> {
        let validated =
            self.validate_generated_chapter_for_checkpoint(session, content, chapter_outline)?;
        let word_count =
            i32::try_from(validated.normalized_content.chars().count()).map_err(|_| {
                AppError::InvalidArgument("生成章节正文字符数超出 i32 范围".to_string())
            })?;
        let updated_chapter =
            a4n_db::chapter_content::finish_generated_chapter_content_record_in_transaction(
                db,
                GeneratedChapterContentRecordInput {
                    project_id: session.project_id,
                    chapter_number: session.chapter_number,
                    existing_chapter_id: session
                        .existing_chapter
                        .as_ref()
                        .map(|chapter| chapter.id),
                    source_outline: source_outline.clone(),
                    title: validated.title,
                    summary: validated.summary,
                    content: validated.normalized_content,
                    word_count,
                    now: Utc::now().naive_utc(),
                },
            )
            .await
            .map_err(chapter_generation_lifecycle_error_to_app_error)?;
        if let Some(trace) = generation_trace {
            self.record_chapter_generation_trace(
                &updated_chapter,
                source_outline.as_ref(),
                chapter_outline,
                content,
                trace,
                db,
            )
            .await?;
        }
        Ok(updated_chapter)
    }
}

mod trace;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_db::chapter_content::GeneratedChapterContentRecordInput;
use a4n_llm::provider::ChatRequest;
use chrono::Utc;

use super::super::ChapterGenerationService;
use super::chapter_generation_lifecycle_error_to_app_error;
use super::session::ChapterGenerationSession;

pub(in crate::chapter_generation::service) struct ChapterGenerationTrace {
    pub(in crate::chapter_generation::service) request: ChatRequest,
    pub(in crate::chapter_generation::service) finish_reason: Option<String>,
    pub(in crate::chapter_generation::service) model_name: String,
}

impl ChapterGenerationService {
    pub(in crate::chapter_generation::service) async fn finish_generated_chapter(
        &self,
        session: &ChapterGenerationSession,
        content: &str,
        chapter_outline: &str,
        _change_description: &str,
        generation_trace: Option<ChapterGenerationTrace>,
    ) -> Result<chapter::Model, AppError> {
        let validated =
            self.validate_generated_chapter_for_checkpoint(session, content, chapter_outline)?;

        let word_count =
            i32::try_from(validated.normalized_content.chars().count()).map_err(|_| {
                AppError::InvalidArgument("生成章节正文字符数超出 i32 范围".to_string())
            })?;
        let now = Utc::now().naive_utc();
        let updated_chapter = a4n_db::chapter_content::finish_generated_chapter_content_record(
            self.db.as_ref(),
            GeneratedChapterContentRecordInput {
                project_id: session.project_id,
                chapter_number: session.chapter_number,
                existing_chapter_id: session.existing_chapter.as_ref().map(|chapter| chapter.id),
                title: validated.title.clone(),
                summary: validated.summary.clone(),
                content: validated.normalized_content.clone(),
                word_count,
                now,
            },
        )
        .await
        .map_err(chapter_generation_lifecycle_error_to_app_error)?;
        if let Some(trace) = generation_trace {
            self.record_chapter_generation_trace(&updated_chapter, chapter_outline, content, trace)
                .await?;
        }

        Ok(updated_chapter)
    }
}

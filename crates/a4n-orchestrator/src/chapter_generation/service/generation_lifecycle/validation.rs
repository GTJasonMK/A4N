use a4n_core::error::AppError;

use super::super::ChapterGenerationService;
use super::session::ChapterGenerationSession;
use crate::chapter_generation::service::response_json::{
    normalize_generated_chapter_content_layout, normalize_generated_chapter_response,
};

mod metadata;
use metadata::generated_chapter_metadata;

impl ChapterGenerationService {
    pub(in crate::chapter_generation) fn validate_generated_chapter_for_checkpoint(
        &self,
        session: &ChapterGenerationSession,
        content: &str,
        chapter_outline: &str,
    ) -> Result<ValidatedGeneratedChapter, AppError> {
        let metadata = generated_chapter_metadata(session, chapter_outline, content);
        let payload = normalize_generated_chapter_response(content, &metadata)?;
        let normalized_content = normalize_generated_chapter_content_layout(&payload.content);
        let normalized_content = normalized_content.trim();
        if normalized_content.is_empty() {
            return Err(AppError::InvalidArgument(
                "LLM 未返回可用章节正文，未写入正文".to_string(),
            ));
        }
        Ok(ValidatedGeneratedChapter {
            title: payload.title,
            summary: payload.summary,
            normalized_content: normalized_content.to_string(),
        })
    }
}

pub(in crate::chapter_generation) struct ValidatedGeneratedChapter {
    pub(in crate::chapter_generation) title: String,
    pub(in crate::chapter_generation) summary: String,
    pub(in crate::chapter_generation) normalized_content: String,
}

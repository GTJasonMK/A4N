use super::super::super::truncate_text_chars;
use super::super::rules::metadata_str;
use a4n_domain::rag::RagDataType;
use a4n_rag_core::metadata::metadata_i64;
use a4n_rag_core::vector_store::SearchResult;

const GENERATION_RAG_PASSAGE_CHAR_LIMIT: usize = 460;

pub(in crate::chapter_generation::rag_context) enum GenerationRagSection {
    Continuity,
    Summary,
    Passage,
}

pub(in crate::chapter_generation::rag_context) struct FormattedRagResult {
    pub(in crate::chapter_generation::rag_context) line: String,
    pub(in crate::chapter_generation::rag_context) section: GenerationRagSection,
}

pub(in crate::chapter_generation::rag_context) fn format_generation_rag_result(
    result: &SearchResult,
    current_chapter_number: i32,
) -> Option<FormattedRagResult> {
    let data_type = RagDataType::parse(metadata_str(&result.metadata, "data_type")?)?;
    let section = generation_rag_section(data_type)?;

    let source_chapter = i32::try_from(metadata_i64(&result.metadata, "chapter_number")?).ok()?;
    if source_chapter <= 0 || source_chapter >= current_chapter_number {
        return None;
    }

    let content = result.content.trim();
    if content.is_empty() {
        return None;
    }

    let title = metadata_str(&result.metadata, "title")?;
    let label = data_type.display_name();
    let score = format!("，相关度{:.3}", result.score);
    let line = format!(
        "- 第{}章《{}》[{}{}]: {}",
        source_chapter,
        title,
        label,
        score,
        truncate_text_chars(content, GENERATION_RAG_PASSAGE_CHAR_LIMIT)
    );

    Some(FormattedRagResult { line, section })
}

fn generation_rag_section(data_type: RagDataType) -> Option<GenerationRagSection> {
    match data_type {
        RagDataType::KeyEvent
        | RagDataType::CharacterState
        | RagDataType::CharacterAttributeEvent => Some(GenerationRagSection::Continuity),
        RagDataType::ChapterSummary | RagDataType::ChapterMetadata => {
            Some(GenerationRagSection::Summary)
        }
        RagDataType::Chapter => Some(GenerationRagSection::Passage),
        _ => None,
    }
}

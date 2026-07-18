use super::super::super::truncate_text_chars;
use super::super::rules::{metadata_i64, metadata_str};
use a4n_core::rag::RagDataType;
use a4n_rag_core::vector_store::SearchResult;

const GENERATION_RAG_PASSAGE_CHAR_LIMIT: usize = 460;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::chapter_generation::rag_context) enum GenerationRagSection {
    Continuity,
    Summary,
    Passage,
}

pub(in crate::chapter_generation::rag_context) struct FormattedRagResult {
    pub(in crate::chapter_generation::rag_context) is_summary: bool,
    pub(in crate::chapter_generation::rag_context) line: String,
    pub(in crate::chapter_generation::rag_context) section: GenerationRagSection,
}

pub(in crate::chapter_generation::rag_context) fn format_generation_rag_result(
    result: &SearchResult,
    project_id: i64,
    current_chapter_number: i32,
) -> Option<FormattedRagResult> {
    let metadata_project_id = metadata_i64(&result.metadata, "project_id")?;
    if metadata_project_id != project_id {
        return None;
    }

    let data_type = metadata_str(&result.metadata, "data_type")?;
    if !is_generation_rag_data_type(data_type) {
        return None;
    }

    let source_chapter = i32::try_from(metadata_i64(&result.metadata, "chapter_number")?).ok()?;
    if source_chapter <= 0 || source_chapter >= current_chapter_number {
        return None;
    }

    let content = result.content.trim();
    if content.is_empty() {
        return None;
    }

    let title = metadata_str(&result.metadata, "title")?;
    let label = generation_rag_type_label(data_type);
    let score = if result.score.is_finite() {
        format!("，相关度{:.3}", result.score)
    } else {
        String::new()
    };
    let line = format!(
        "- 第{}章《{}》[{}{}]: {}",
        source_chapter,
        title,
        label,
        score,
        truncate_text_chars(content, GENERATION_RAG_PASSAGE_CHAR_LIMIT)
    );

    Some(FormattedRagResult {
        is_summary: data_type == RagDataType::ChapterSummary.as_str(),
        line,
        section: generation_rag_section(data_type),
    })
}

fn generation_rag_section(data_type: &str) -> GenerationRagSection {
    match data_type {
        "foreshadowing" | "character_state" | "character_attribute_event" | "key_event" => {
            GenerationRagSection::Continuity
        }
        "chapter_summary" | "chapter_metadata" => GenerationRagSection::Summary,
        _ => GenerationRagSection::Passage,
    }
}

fn is_generation_rag_data_type(data_type: &str) -> bool {
    matches!(
        data_type,
        "chapter_summary"
            | "chapter"
            | "chapter_metadata"
            | "key_event"
            | "foreshadowing"
            | "character_state"
            | "character_attribute_event"
    )
}

fn generation_rag_type_label(data_type: &str) -> &'static str {
    match RagDataType::parse(data_type) {
        Some(data_type) => data_type.display_name(),
        None => "参考",
    }
}

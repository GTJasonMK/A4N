use super::super::reference_context::truncate_text_chars;
use super::{metadata_i64, metadata_str};
use a4n_core::rag::RagDataType;
use a4n_engine::rag::vector_store::SearchResult;
use std::collections::HashSet;

pub(crate) const GENERATION_RAG_CONTEXT_CHAR_LIMIT: usize = 3_600;
const GENERATION_RAG_PASSAGE_CHAR_LIMIT: usize = 460;
const GENERATION_RAG_CONTINUITY_RATIO: f32 = 0.45;
const GENERATION_RAG_SUMMARY_RATIO: f32 = 0.35;
const GENERATION_RAG_PASSAGE_RATIO: f32 = 0.20;
const GENERATION_RAG_MAX_CONTINUITY_RESULTS: usize = 4;
const GENERATION_RAG_MAX_SUMMARY_RESULTS: usize = 3;
pub(crate) const GENERATION_RAG_MAX_PASSAGE_RESULTS: usize = 3;

pub(crate) fn format_generation_rag_context(
    results: &[SearchResult],
    project_id: i64,
    chapter_number: i32,
) -> FormattedRagContext {
    let mut seen = HashSet::new();
    let mut continuity_candidates = Vec::new();
    let mut summary_candidates = Vec::new();
    let mut passage_candidates = Vec::new();

    for result in results {
        if !seen.insert(result.id.clone()) {
            continue;
        }
        let Some(item) = format_generation_rag_result(result, project_id, chapter_number) else {
            continue;
        };
        let section = if item.is_summary {
            GenerationRagSection::Summary
        } else {
            item.section
        };
        match section {
            GenerationRagSection::Continuity => {
                if continuity_candidates.len() < GENERATION_RAG_MAX_CONTINUITY_RESULTS {
                    continuity_candidates.push(item.line);
                }
            }
            GenerationRagSection::Summary => {
                if summary_candidates.len() < GENERATION_RAG_MAX_SUMMARY_RESULTS {
                    summary_candidates.push(item.line);
                }
            }
            GenerationRagSection::Passage => {
                if passage_candidates.len() < GENERATION_RAG_MAX_PASSAGE_RESULTS {
                    passage_candidates.push(item.line);
                }
            }
        }

        if continuity_candidates.len() >= GENERATION_RAG_MAX_CONTINUITY_RESULTS
            && summary_candidates.len() >= GENERATION_RAG_MAX_SUMMARY_RESULTS
            && passage_candidates.len() >= GENERATION_RAG_MAX_PASSAGE_RESULTS
        {
            break;
        }
    }

    let continuity_budget = rag_section_budget(
        GENERATION_RAG_CONTEXT_CHAR_LIMIT,
        GENERATION_RAG_CONTINUITY_RATIO,
    );
    let summary_base_budget = rag_section_budget(
        GENERATION_RAG_CONTEXT_CHAR_LIMIT,
        GENERATION_RAG_SUMMARY_RATIO,
    );
    let passage_base_budget = rag_section_budget(
        GENERATION_RAG_CONTEXT_CHAR_LIMIT,
        GENERATION_RAG_PASSAGE_RATIO,
    );

    let continuity = take_budgeted_rag_lines(
        continuity_candidates,
        continuity_budget,
        GENERATION_RAG_MAX_CONTINUITY_RESULTS,
    );
    let continuity_spare = continuity_budget.saturating_sub(line_block_char_count(&continuity));
    let summary_budget = summary_base_budget + (continuity_spare * 3 / 5);
    let summaries = take_budgeted_rag_lines(
        summary_candidates,
        summary_budget,
        GENERATION_RAG_MAX_SUMMARY_RESULTS,
    );
    let passage_budget = passage_base_budget
        + (continuity_spare * 2 / 5)
        + summary_budget.saturating_sub(line_block_char_count(&summaries));
    let passages = take_budgeted_rag_lines(
        passage_candidates,
        passage_budget,
        GENERATION_RAG_MAX_PASSAGE_RESULTS,
    );

    if continuity.is_empty() && summaries.is_empty() && passages.is_empty() {
        return FormattedRagContext {
            text: None,
            continuity_count: 0,
            summary_count: 0,
            chunk_count: 0,
        };
    }

    let mut lines = vec!["## 相关检索参考".to_string()];
    let continuity_count = continuity.len();
    let summary_count = summaries.len();
    let chunk_count = passages.len();
    append_generation_rag_section(&mut lines, "### 关键连续性", &continuity);
    if !summaries.is_empty() {
        if lines.len() > 1 {
            lines.push(String::new());
        }
        append_generation_rag_section(&mut lines, "### 相关摘要", &summaries);
    }
    if !passages.is_empty() {
        if lines.len() > 1 {
            lines.push(String::new());
        }
        append_generation_rag_section(&mut lines, "### 参考片段", &passages);
    }

    FormattedRagContext {
        text: Some(truncate_text_chars(
            &lines.join("\n"),
            GENERATION_RAG_CONTEXT_CHAR_LIMIT,
        )),
        continuity_count,
        summary_count,
        chunk_count,
    }
}

fn rag_section_budget(total: usize, ratio: f32) -> usize {
    ((total as f32 * ratio).round() as usize).max(240)
}

fn take_budgeted_rag_lines(
    candidates: Vec<String>,
    budget: usize,
    max_count: usize,
) -> Vec<String> {
    let mut selected = Vec::new();
    let mut used = 0usize;
    for line in candidates.into_iter().take(max_count) {
        let newline_cost = usize::from(!selected.is_empty());
        let line_len = line.chars().count();
        if used + newline_cost + line_len <= budget {
            used += newline_cost + line_len;
            selected.push(line);
            continue;
        }

        let remaining = budget.saturating_sub(used + newline_cost);
        if remaining >= 120 {
            selected.push(truncate_text_chars(&line, remaining));
        }
        break;
    }
    selected
}

fn line_block_char_count(lines: &[String]) -> usize {
    lines
        .iter()
        .enumerate()
        .map(|(idx, line)| line.chars().count() + usize::from(idx > 0))
        .sum()
}

fn append_generation_rag_section(lines: &mut Vec<String>, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    lines.push(title.to_string());
    lines.extend(items.iter().cloned());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenerationRagSection {
    Continuity,
    Summary,
    Passage,
}

pub(crate) struct FormattedRagResult {
    pub(crate) is_summary: bool,
    pub(crate) line: String,
    section: GenerationRagSection,
}

pub(crate) struct FormattedRagContext {
    pub(crate) text: Option<String>,
    pub(crate) continuity_count: usize,
    pub(crate) summary_count: usize,
    pub(crate) chunk_count: usize,
}

pub(crate) fn format_generation_rag_result(
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

    let source_chapter = metadata_i64(&result.metadata, "chapter_number")? as i32;
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

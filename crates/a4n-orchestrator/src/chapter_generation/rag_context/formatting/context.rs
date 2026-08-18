use super::super::super::truncate_text_chars;
use super::result::{GenerationRagSection, format_generation_rag_result};
use a4n_rag_core::vector_store::SearchResult;

pub(in crate::chapter_generation::rag_context) const GENERATION_RAG_CONTEXT_CHAR_LIMIT: usize =
    3_600;
const GENERATION_RAG_CONTINUITY_RATIO: f32 = 0.45;
const GENERATION_RAG_SUMMARY_RATIO: f32 = 0.35;
const GENERATION_RAG_PASSAGE_RATIO: f32 = 0.20;
const GENERATION_RAG_MAX_CONTINUITY_RESULTS: usize = 4;
const GENERATION_RAG_MAX_SUMMARY_RESULTS: usize = 3;
pub(in crate::chapter_generation::rag_context) const GENERATION_RAG_MAX_PASSAGE_RESULTS: usize = 3;

pub(in crate::chapter_generation::rag_context) fn format_generation_rag_context(
    results: &[SearchResult],
    chapter_number: i32,
) -> Option<String> {
    let mut continuity_candidates = Vec::new();
    let mut summary_candidates = Vec::new();
    let mut passage_candidates = Vec::new();

    for result in results {
        let Some(item) = format_generation_rag_result(result, chapter_number) else {
            continue;
        };
        match item.section {
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
        return None;
    }

    let mut lines = vec!["## 相关检索参考".to_string()];
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

    Some(truncate_text_chars(
        &lines.join("\n"),
        GENERATION_RAG_CONTEXT_CHAR_LIMIT,
    ))
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

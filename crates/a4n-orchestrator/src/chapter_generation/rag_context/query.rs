use super::super::{
    compact_json_text, is_meaningful_json_value, outline_character_names, truncate_text_chars,
};
use a4n_core::models::{chapter::chapter_outline, character::foreshadowing_index};
use a4n_core::text_location::extract_scene_location;

const GENERATION_RAG_MAX_QUERIES: usize = 7;
const GENERATION_RAG_QUERY_CHAR_LIMIT: usize = 1_800;

pub(crate) struct GenerationRagQuery {
    pub(crate) text: String,
    pub(crate) weight: f32,
}

fn push_generation_rag_query(queries: &mut Vec<GenerationRagQuery>, text: String, weight: f32) {
    let text = text.trim();
    if text.is_empty() || queries.iter().any(|query| query.text == text) {
        return;
    }
    queries.push(GenerationRagQuery {
        text: text.to_string(),
        weight,
    });
}

pub(super) fn build_generation_rag_queries(
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
    pending_foreshadowing: &[foreshadowing_index::Model],
) -> Vec<GenerationRagQuery> {
    let mut parts = Vec::new();

    if let Some(outline) = outline {
        parts.push(format!("第{}章", outline.chapter_number));
        if let Some(title) = outline
            .title
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            parts.push(format!("章节标题: {title}"));
        }
        if let Some(summary) = outline
            .summary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            parts.push(format!("章节摘要: {}", truncate_text_chars(summary, 360)));
        }
        for (label, value) in [
            ("节拍", outline.beats.as_ref()),
            ("关键场景", outline.key_scenes.as_ref()),
            ("涉及角色", outline.characters_involved.as_ref()),
            ("本章埋设伏笔", outline.foreshadowing_setup.as_ref()),
            ("本章回收伏笔", outline.foreshadowing_payoff.as_ref()),
        ] {
            if let Some(text) = value
                .filter(|value| is_meaningful_json_value(value))
                .map(|value| compact_json_text(value, 320))
                .filter(|text| !text.trim().is_empty())
            {
                parts.push(format!("{label}: {text}"));
            }
        }
    } else {
        parts.push(truncate_text_chars(outline_text, 700));
    }

    if let Some(instruction) = writing_instruction
        .map(str::trim)
        .filter(|instruction| !instruction.is_empty())
    {
        parts.push(format!(
            "写作要点: {}",
            truncate_text_chars(instruction, 280)
        ));
    }

    let main_query = truncate_text_chars(
        &dedup_non_empty_lines(parts).join("\n"),
        GENERATION_RAG_QUERY_CHAR_LIMIT,
    );
    let mut queries = Vec::new();
    push_generation_rag_query(&mut queries, main_query, 1.0);
    for name in outline_character_names(outline).into_iter().take(3) {
        push_generation_rag_query(
            &mut queries,
            format!("角色 {name} 的行动、立场、关系和状态变化"),
            0.85,
        );
    }
    for item in pending_foreshadowing.iter().take(3) {
        push_generation_rag_query(
            &mut queries,
            format!("伏笔: {}", truncate_text_chars(&item.description, 180)),
            0.95,
        );
    }
    if let Some(location) = extract_scene_location(outline_text) {
        push_generation_rag_query(
            &mut queries,
            format!("场景 {location} 中发生的事件和相关设定"),
            0.75,
        );
    }
    queries.truncate(GENERATION_RAG_MAX_QUERIES);
    queries
}

fn dedup_non_empty_lines(lines: Vec<String>) -> Vec<String> {
    let mut deduped: Vec<String> = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || deduped.iter().any(|candidate| candidate.as_str() == line) {
            continue;
        }
        deduped.push(line.to_string());
    }
    deduped
}

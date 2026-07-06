use super::super::reference_context::{
    compact_json_text, is_meaningful_json_value, outline_character_names, truncate_text_chars,
};
use a4n_core::models::{chapter::chapter_outline, character::foreshadowing_index};
use std::collections::HashSet;

const GENERATION_RAG_MAX_QUERIES: usize = 7;
const GENERATION_RAG_QUERY_CHAR_LIMIT: usize = 1_800;

#[derive(Debug, Clone, Default)]
pub(crate) struct GenerationRagQueries {
    pub(crate) main_query: String,
    pub(crate) character_queries: Vec<String>,
    pub(crate) foreshadowing_queries: Vec<String>,
    pub(crate) location_query: Option<String>,
}

impl GenerationRagQueries {
    pub(crate) fn is_empty(&self) -> bool {
        self.weighted_queries().is_empty()
    }

    pub(crate) fn weighted_queries(&self) -> Vec<WeightedGenerationQuery<'_>> {
        let mut queries = Vec::new();
        push_weighted_generation_query(&mut queries, self.main_query.as_str(), 1.0);
        for query in &self.character_queries {
            push_weighted_generation_query(&mut queries, query, 0.85);
        }
        for query in &self.foreshadowing_queries {
            push_weighted_generation_query(&mut queries, query, 0.95);
        }
        if let Some(query) = self.location_query.as_deref() {
            push_weighted_generation_query(&mut queries, query, 0.75);
        }
        queries.truncate(GENERATION_RAG_MAX_QUERIES);
        queries
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WeightedGenerationQuery<'a> {
    pub(crate) text: &'a str,
    pub(crate) weight: f32,
}

fn push_weighted_generation_query<'a>(
    queries: &mut Vec<WeightedGenerationQuery<'a>>,
    text: &'a str,
    weight: f32,
) {
    let text = text.trim();
    if text.is_empty() || queries.iter().any(|query| query.text == text) {
        return;
    }
    queries.push(WeightedGenerationQuery { text, weight });
}

pub(crate) fn build_generation_rag_queries(
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
    pending_foreshadowing: &[foreshadowing_index::Model],
) -> GenerationRagQueries {
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
    let character_queries = outline_character_names(outline)
        .into_iter()
        .take(3)
        .map(|name| format!("角色 {name} 的行动、立场、关系和状态变化"))
        .collect();
    let foreshadowing_queries = pending_foreshadowing
        .iter()
        .take(3)
        .map(|item| format!("伏笔: {}", truncate_text_chars(&item.description, 180)))
        .filter(|text| !text.trim().is_empty())
        .collect();
    let location_query = extract_generation_location(outline_text)
        .map(|location| format!("场景 {location} 中发生的事件和相关设定"));

    GenerationRagQueries {
        main_query,
        character_queries,
        foreshadowing_queries,
        location_query,
    }
}

pub(super) fn extract_generation_location(text: &str) -> Option<String> {
    for cue in ["来到", "抵达", "前往", "回到"] {
        if let Some(location) = extract_location_after_cue(text, cue, false) {
            return Some(location);
        }
    }
    extract_location_after_cue(text, "在", true)
}

fn extract_location_after_cue(
    text: &str,
    cue: &str,
    require_location_suffix: bool,
) -> Option<String> {
    let chars = text.chars().collect::<Vec<_>>();
    let cue_chars = cue.chars().collect::<Vec<_>>();
    if cue_chars.is_empty() || chars.len() <= cue_chars.len() {
        return None;
    }

    for start in 0..=chars.len().saturating_sub(cue_chars.len()) {
        if chars[start..start + cue_chars.len()] != cue_chars {
            continue;
        }
        let mut location = String::new();
        let mut has_location_suffix = false;
        for ch in chars.iter().skip(start + cue_chars.len()) {
            if location.chars().count() >= 10 || is_location_stop_char(*ch) {
                break;
            }
            if location.chars().count() >= 2 && is_location_action_start(*ch) {
                break;
            }
            if matches!(*ch, '中' | '里' | '内' | '上' | '下' | '旁' | '边') {
                has_location_suffix = true;
                break;
            }
            location.push(*ch);
        }
        let location = location.trim_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, '《' | '》' | '"' | '\'' | '“' | '”' | '「' | '」')
        });
        if (2..=10).contains(&location.chars().count())
            && (!require_location_suffix || has_location_suffix)
        {
            return Some(location.to_string());
        }
    }
    None
}

fn is_location_stop_char(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '，' | '。'
                | '、'
                | '；'
                | '：'
                | '！'
                | '？'
                | ','
                | '.'
                | ';'
                | ':'
                | '!'
                | '?'
                | '\n'
                | '\r'
        )
}

fn is_location_action_start(ch: char) -> bool {
    matches!(
        ch,
        '调' | '确' | '寻' | '会' | '发' | '阻' | '救' | '追' | '见' | '问' | '记' | '查'
    )
}

fn dedup_non_empty_lines(lines: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() || !seen.insert(line.to_string()) {
            continue;
        }
        deduped.push(line.to_string());
    }
    deduped
}

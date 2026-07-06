mod rag_context;
mod reference_context;

pub(super) use self::rag_context::{
    GenerationRagContext, PreparedGenerationRagEmbeddingRequest, build_generation_rag_context,
    build_generation_rag_context_with_embeddings, prepare_generation_rag_embedding_request,
};
use self::reference_context::is_meaningful_json_value;
pub(super) use self::reference_context::resolve_generation_reference_context;
use crate::AppState;
use a4n_core::models::chapter::chapter_outline;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;

const DEFAULT_CHAPTER_WORD_COUNT_MIN: i32 = 4_500;
const DEFAULT_CHAPTER_WORD_COUNT_MAX: i32 = 6_000;
const WORD_COUNT_TARGET_MIN_RATIO_NUMERATOR: i32 = 9;
const WORD_COUNT_TARGET_MIN_RATIO_DENOMINATOR: i32 = 10;
const WORD_COUNT_TARGET_MAX_RATIO_NUMERATOR: i32 = 11;
const WORD_COUNT_TARGET_MAX_RATIO_DENOMINATOR: i32 = 10;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub(super) struct ChapterGenerationWordCountRange {
    pub(super) min: i32,
    pub(super) max: i32,
}

impl ChapterGenerationWordCountRange {
    fn label(self) -> String {
        format!("{}-{} 字", self.min, self.max)
    }
}

fn format_chapter_outline_for_generation(
    outline: &chapter_outline::Model,
    chapter_title: &str,
    word_count_range: Option<ChapterGenerationWordCountRange>,
) -> String {
    let mut lines = vec![
        "## 当前任务".to_string(),
        format!("章节: 第{}章《{}》", outline.chapter_number, chapter_title),
    ];

    push_optional_text(&mut lines, "大纲摘要", outline.summary.as_deref());
    if word_count_range.is_none() {
        push_optional_i32(&mut lines, "目标字数", outline.word_count_target);
    }
    push_optional_f32(&mut lines, "节奏评分", outline.pacing_score);
    push_optional_json(&mut lines, "节拍安排", outline.beats.as_ref());
    push_optional_json(&mut lines, "关键场景", outline.key_scenes.as_ref());
    push_optional_json(&mut lines, "涉及角色", outline.characters_involved.as_ref());
    push_optional_json(
        &mut lines,
        "本章埋设伏笔",
        outline.foreshadowing_setup.as_ref(),
    );
    push_optional_json(
        &mut lines,
        "本章回收伏笔",
        outline.foreshadowing_payoff.as_ref(),
    );

    lines.join("\n")
}

pub(super) fn merge_generation_outline_text(
    chapter_number: i32,
    db_outline: Option<&chapter_outline::Model>,
    provided_outline: Option<&str>,
    writing_instruction: Option<&str>,
    chapter_title: Option<&str>,
    word_count_range: Option<ChapterGenerationWordCountRange>,
) -> Result<String, String> {
    let mut sections = Vec::new();
    let mut has_outline_source = false;

    if let Some(outline) = db_outline {
        let title = chapter_title
            .and_then(non_empty_trimmed)
            .ok_or_else(|| format!("第 {chapter_number} 章缺少标题，无法构建章节生成大纲"))?;
        sections.push(format_chapter_outline_for_generation(
            outline,
            title,
            word_count_range,
        ));
        has_outline_source = true;
    }

    if let Some(outline) = provided_outline.map(str::trim).filter(|s| !s.is_empty()) {
        let is_duplicate = db_outline
            .and_then(|db| db.summary.as_deref())
            .is_some_and(|summary| summary.trim() == outline);
        if !is_duplicate {
            sections.push(format!("## 用户补充章纲\n{outline}"));
        }
        has_outline_source = true;
    }

    if !has_outline_source {
        return Err(format!("第 {chapter_number} 章缺少章节大纲，无法生成正文"));
    }

    if let Some(instruction) = writing_instruction.map(str::trim).filter(|s| !s.is_empty()) {
        sections.push(format!("## 写作指令\n{instruction}"));
    }

    if let Some(range) = word_count_range {
        sections.push(format_generation_word_count_requirement(range));
    }

    Ok(sections.join("\n\n"))
}

pub(super) fn resolve_generation_word_count_range(
    outline: Option<&chapter_outline::Model>,
    min_word_count: Option<i32>,
    max_word_count: Option<i32>,
) -> Result<ChapterGenerationWordCountRange, String> {
    match (min_word_count, max_word_count) {
        (Some(min), Some(max)) => validate_generation_word_count_range(min, max),
        (Some(_), None) | (None, Some(_)) => {
            Err("正文字数范围必须同时提供最小字数和最大字数".to_string())
        }
        (None, None) => Ok(default_generation_word_count_range(outline)),
    }
}

fn default_generation_word_count_range(
    outline: Option<&chapter_outline::Model>,
) -> ChapterGenerationWordCountRange {
    let Some(target) = outline.and_then(|outline| outline.word_count_target) else {
        return ChapterGenerationWordCountRange {
            min: DEFAULT_CHAPTER_WORD_COUNT_MIN,
            max: DEFAULT_CHAPTER_WORD_COUNT_MAX,
        };
    };
    if target <= 0 {
        return ChapterGenerationWordCountRange {
            min: DEFAULT_CHAPTER_WORD_COUNT_MIN,
            max: DEFAULT_CHAPTER_WORD_COUNT_MAX,
        };
    }
    let min = scaled_i32_floor(
        target,
        WORD_COUNT_TARGET_MIN_RATIO_NUMERATOR,
        WORD_COUNT_TARGET_MIN_RATIO_DENOMINATOR,
    )
    .max(1);
    let max = scaled_i32_ceil(
        target,
        WORD_COUNT_TARGET_MAX_RATIO_NUMERATOR,
        WORD_COUNT_TARGET_MAX_RATIO_DENOMINATOR,
    )
    .max(min);
    ChapterGenerationWordCountRange { min, max }
}

fn scaled_i32_floor(value: i32, numerator: i32, denominator: i32) -> i32 {
    value.saturating_mul(numerator) / denominator
}

fn scaled_i32_ceil(value: i32, numerator: i32, denominator: i32) -> i32 {
    value
        .saturating_mul(numerator)
        .saturating_add(denominator - 1)
        / denominator
}

fn validate_generation_word_count_range(
    min: i32,
    max: i32,
) -> Result<ChapterGenerationWordCountRange, String> {
    if min <= 0 {
        return Err("正文字数最小值必须大于 0".to_string());
    }
    if max <= 0 {
        return Err("正文字数最大值必须大于 0".to_string());
    }
    if max < min {
        return Err("正文字数最大值不能小于最小值".to_string());
    }
    Ok(ChapterGenerationWordCountRange { min, max })
}

fn format_generation_word_count_requirement(range: ChapterGenerationWordCountRange) -> String {
    format!(
        "## 正文字数要求\n最小字数: {}\n最大字数: {}\n要求: 章节正文纯文本必须控制在 {} 之间；按正文字符数估算，不要输出 word_count 字段、JSON 字段名、标题字段或摘要字段，不要用重复、注水或解释性段落凑字。",
        range.min,
        range.max,
        range.label()
    )
}

pub(super) async fn find_generation_outline(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<chapter_outline::Model>, String> {
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())
}

fn push_optional_text(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) {
        lines.push(format!("{label}: {value}"));
    }
}

fn push_optional_i32(lines: &mut Vec<String>, label: &str, value: Option<i32>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {value}"));
    }
}

fn push_optional_f32(lines: &mut Vec<String>, label: &str, value: Option<f32>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {:.2}", value));
    }
}

fn push_optional_json(lines: &mut Vec<String>, label: &str, value: Option<&Value>) {
    let Some(value) = value.filter(|value| is_meaningful_json_value(value)) else {
        return;
    };
    let text = serde_json::to_string_pretty(value)
        .expect("serde_json::Value pretty serialization must succeed");
    lines.push(format!("{label}:\n{text}"));
}

pub(super) fn required_generation_chapter_title(
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    title_hint: Option<&str>,
) -> Result<String, String> {
    title_hint
        .and_then(non_empty_trimmed)
        .map(str::to_string)
        .or_else(|| {
            outline
                .and_then(|outline| outline.title.as_deref())
                .and_then(non_empty_trimmed)
                .map(str::to_string)
        })
        .ok_or_else(|| format!("第 {chapter_number} 章缺少标题，无法生成正文"))
}

pub(super) fn upsert_chapter_generation_variables(
    variables: &mut HashMap<String, Value>,
    chapter_number: i32,
    chapter_title: &str,
    outline_text: &str,
    writing_instruction: Option<&str>,
    reference_context: Option<&str>,
    rag_context: Option<&str>,
    word_count_range: ChapterGenerationWordCountRange,
) {
    variables.insert("chapter_number".to_string(), json!(chapter_number));
    variables.insert("chapter_title".to_string(), json!(chapter_title));
    variables.insert("chapter_outline".to_string(), json!(outline_text));
    variables.insert(
        "chapter_word_count_min".to_string(),
        json!(word_count_range.min),
    );
    variables.insert(
        "chapter_word_count_max".to_string(),
        json!(word_count_range.max),
    );
    variables.insert(
        "chapter_word_count_range".to_string(),
        json!(word_count_range.label()),
    );
    upsert_optional_text_variable(variables, "writing_instruction", writing_instruction);
    upsert_optional_text_variable(variables, "generation_reference_context", reference_context);
    upsert_optional_text_variable(variables, "generation_rag_context", rag_context);
}

fn upsert_optional_text_variable(
    variables: &mut HashMap<String, Value>,
    key: &str,
    value: Option<&str>,
) {
    match value.and_then(non_empty_trimmed) {
        Some(value) => {
            variables.insert(key.to_string(), json!(value));
        }
        None => {
            variables.remove(key);
        }
    }
}

pub(super) fn non_empty_trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
}

#[cfg(test)]
mod tests {
    use super::reference_context::{
        format_character_reference_line, format_foreshadowing_reference_line,
        select_generation_characters, select_generation_foreshadowing,
    };
    use super::*;
    use a4n_core::models::{character::foreshadowing_index, project::blueprint_character};
    use serde_json::json;

    fn generation_outline(title: Option<&str>, summary: Option<&str>) -> chapter_outline::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter_outline::Model {
            id: 1,
            project_id: 1,
            part_id: None,
            chapter_number: 2,
            title: title.map(str::to_string),
            summary: summary.map(str::to_string),
            beats: None,
            key_scenes: None,
            pacing_score: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: None,
            status: "draft".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    fn outline_with_characters() -> chapter_outline::Model {
        let mut outline =
            generation_outline(Some("玄武暗线"), Some("林岚与陈恪前往玄武门调查旧案。"));
        outline.chapter_number = 6;
        outline.characters_involved = Some(json!(["林岚", "陈恪"]));
        outline
    }

    fn foreshadowing_item(payoff_chapter: Option<i32>) -> foreshadowing_index::Model {
        let now = chrono::Utc::now().naive_utc();
        foreshadowing_index::Model {
            id: 1,
            project_id: 1,
            description: "旧钟在午夜倒走".to_string(),
            setup_chapter: 2,
            payoff_chapter,
            status: "pending".to_string(),
            related_characters: None,
            importance: 4,
            created_at: now,
            updated_at: now,
        }
    }

    fn blueprint_character(name: &str) -> blueprint_character::Model {
        let now = chrono::Utc::now().naive_utc();
        blueprint_character::Model {
            id: 1,
            project_id: 1,
            name: name.to_string(),
            character_type: "supporting".to_string(),
            gender: Some("女".to_string()),
            age: Some("二十岁".to_string()),
            appearance: Some(format!("{name}的外貌约束")),
            personality: Some("谨慎".to_string()),
            background: Some(format!("{name}的背景约束")),
            motivation: Some("寻找真相".to_string()),
            arc_summary: Some("从旁观到介入".to_string()),
            relationship_to_protagonist: Some("盟友".to_string()),
            arc_stages: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn required_generation_chapter_title_rejects_missing_title() {
        let outline = generation_outline(Some("  "), Some("旧城暗线浮出水面"));

        let err = required_generation_chapter_title(2, Some(&outline), None)
            .expect_err("blank outline title must fail");

        assert!(err.contains("第 2 章缺少标题"));
    }

    #[test]
    fn required_generation_chapter_title_prefers_explicit_hint() {
        let outline = generation_outline(Some("大纲标题"), Some("旧城暗线浮出水面"));

        let title = required_generation_chapter_title(2, Some(&outline), Some("  正文标题  "))
            .expect("explicit title hint should pass");

        assert_eq!(title, "正文标题");
    }

    #[test]
    fn merge_generation_outline_text_rejects_missing_outline_source() {
        let err = merge_generation_outline_text(
            2,
            None,
            Some("  "),
            Some("强调悬疑节奏"),
            Some("星门余烬"),
            None,
        )
        .expect_err("generation outline source must be required");

        assert!(err.contains("第 2 章缺少章节大纲"));
    }

    #[test]
    fn merge_generation_outline_text_formats_required_title() {
        let outline = generation_outline(Some("星门余烬"), Some("旧城暗线浮出水面"));

        let text = merge_generation_outline_text(
            2,
            Some(&outline),
            None,
            Some("  强调悬疑节奏  "),
            Some("星门余烬"),
            None,
        )
        .expect("valid outline should format");

        assert!(text.contains("章节: 第2章《星门余烬》"));
        assert!(text.contains("大纲摘要: 旧城暗线浮出水面"));
        assert!(text.contains("## 写作指令\n强调悬疑节奏"));
    }

    #[test]
    fn resolves_explicit_generation_word_count_range() {
        let range = resolve_generation_word_count_range(None, Some(3_000), Some(4_200))
            .expect("explicit range should be valid");

        assert_eq!(
            range,
            ChapterGenerationWordCountRange {
                min: 3_000,
                max: 4_200,
            }
        );
    }

    #[test]
    fn rejects_incomplete_generation_word_count_range() {
        let err = resolve_generation_word_count_range(None, Some(3_000), None)
            .expect_err("incomplete range should be rejected");

        assert!(err.contains("同时提供"));
    }

    #[test]
    fn derives_generation_word_count_range_from_outline_target() {
        let mut outline = outline_with_characters();
        outline.word_count_target = Some(5_000);

        let range = resolve_generation_word_count_range(Some(&outline), None, None)
            .expect("outline target should derive a range");

        assert_eq!(
            range,
            ChapterGenerationWordCountRange {
                min: 4_500,
                max: 5_500,
            }
        );
    }

    #[test]
    fn merge_generation_outline_text_adds_word_count_requirement_once() {
        let outline = outline_with_characters();
        let text = merge_generation_outline_text(
            6,
            Some(&outline),
            None,
            None,
            Some("玄武暗线"),
            Some(ChapterGenerationWordCountRange {
                min: 4_000,
                max: 5_000,
            }),
        )
        .expect("outline should merge");

        assert!(text.contains("## 正文字数要求"));
        assert!(text.contains("最小字数: 4000"));
        assert!(text.contains("最大字数: 5000"));
        assert!(!text.contains("目标字数:"));
    }

    #[test]
    fn foreshadowing_reference_omits_missing_payoff_placeholder() {
        let item = foreshadowing_item(None);

        let line = format_foreshadowing_reference_line(&&item);

        assert_eq!(line, "- [重要度4] 第2章埋设：旧钟在午夜倒走");
        assert!(!line.contains("未定"));
    }

    #[test]
    fn foreshadowing_reference_includes_explicit_payoff_chapter() {
        let item = foreshadowing_item(Some(7));

        let line = format_foreshadowing_reference_line(&&item);

        assert_eq!(line, "- [重要度4] 第2章埋设，预计第7章回收：旧钟在午夜倒走");
    }

    #[test]
    fn select_generation_foreshadowing_requires_related_characters_when_filtering() {
        let item_without_related = foreshadowing_item(None);
        let mut item_with_related = foreshadowing_item(None);
        item_with_related.id = 2;
        item_with_related.related_characters = Some(json!(["林岚"]));
        let involved = vec!["林岚".to_string()];
        let items = [item_without_related, item_with_related];

        let selected = select_generation_foreshadowing(&items, 3, &involved);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, 2);
    }

    #[test]
    fn select_generation_foreshadowing_keeps_unfiltered_items_without_related_characters() {
        let item = foreshadowing_item(None);
        let items = [item];

        let selected = select_generation_foreshadowing(&items, 3, &[]);

        assert_eq!(selected.len(), 1);
    }

    #[test]
    fn generation_reference_keeps_all_explicitly_involved_characters() {
        let characters = (0..8)
            .map(|idx| blueprint_character(&format!("角色{idx}")))
            .collect::<Vec<_>>();
        let involved = characters
            .iter()
            .map(|character| character.name.clone())
            .collect::<Vec<_>>();

        let selected = select_generation_characters(&characters, &involved);

        assert_eq!(selected.len(), characters.len());
        assert!(selected.iter().any(|character| character.name == "角色7"));
        let line = format_character_reference_line(&selected[7]);
        assert!(line.contains("外貌"));
        assert!(line.contains("背景"));
        assert!(line.contains("角色7的外貌约束"));
    }
}

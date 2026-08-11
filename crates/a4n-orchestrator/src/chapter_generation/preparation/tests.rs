use super::{
    ChapterGenerationWordCountRange, generation_chapter_title, merge_generation_outline_text,
    reference_context::{
        formatting::{format_character_reference_line, format_foreshadowing_reference_line},
        selection::{select_generation_characters, select_generation_foreshadowing},
    },
    resolve_generation_word_count_range,
};
use a4n_core::models::{
    chapter::chapter_outline, character::foreshadowing_index, project::blueprint_character,
};
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
    let mut outline = generation_outline(Some("玄武暗线"), Some("林岚与陈恪前往玄武门调查旧案。"));
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
fn generation_chapter_title_falls_back_to_chapter_number() {
    let outline = generation_outline(Some("  "), Some("旧城暗线浮出水面"));

    let title = generation_chapter_title(2, Some(&outline), None);

    assert_eq!(title, "第2章");
}

#[test]
fn generation_chapter_title_prefers_explicit_hint() {
    let outline = generation_outline(Some("大纲标题"), Some("旧城暗线浮出水面"));

    let title = generation_chapter_title(2, Some(&outline), Some("  正文标题  "));

    assert_eq!(title, "正文标题");
}

#[test]
fn merge_generation_outline_text_rejects_missing_outline_and_brief() {
    let err = merge_generation_outline_text(2, None, Some("  "), None, Some("星门余烬"), None)
        .expect_err("outline or brief must be required");

    assert!(err.contains("缺少可用章节大纲或本章创作简报"));
}

#[test]
fn merge_generation_outline_text_uses_brief_without_formal_outline() {
    let text = merge_generation_outline_text(
        2,
        None,
        None,
        Some("  林岚在旧仓库发现失踪者留下的录音，并决定隐瞒其中一句话。  "),
        None,
        Some(ChapterGenerationWordCountRange {
            min: 3_000,
            max: 4_000,
        }),
    )
    .expect("brief should be enough to generate without a formal outline");

    assert!(text.contains("章节标题: 第2章"));
    assert!(text.contains("本章创作简报:\n林岚在旧仓库发现失踪者留下的录音"));
    assert!(!text.contains("## 写作指令"));
    assert!(text.contains("## 正文字数要求"));
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

    assert!(text.contains("章节编号: 第2章"));
    assert!(text.contains("章节标题: 星门余烬"));
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
fn rejects_outline_word_count_target_whose_ratio_exceeds_i32() {
    let mut outline = outline_with_characters();
    outline.word_count_target = Some(i32::MAX);

    let err = resolve_generation_word_count_range(Some(&outline), None, None)
        .expect_err("derived word count range overflow must be visible");

    assert!(err.contains("章节目标字数比例计算超过 i32 上限"));
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

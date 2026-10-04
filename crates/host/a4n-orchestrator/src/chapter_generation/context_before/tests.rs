use super::{
    analysis::chapter_analysis_context_line,
    character_state::{character_profile_context_line, character_state_context_line},
    summaries::layered_chapter_summary_lines,
};
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::character::{CharacterProfileRecord, CharacterStateIndexRecord};

fn chapter_for_context(
    chapter_number: i32,
    title: Option<&str>,
    summary: Option<&str>,
    analysis_data: Option<serde_json::Value>,
) -> ChapterRecord {
    let now = chrono::Utc::now().naive_utc();
    ChapterRecord {
        id: i64::from(chapter_number),
        project_id: 1,
        outline_id: None,
        chapter_number,
        title: title.map(str::to_string),
        content: "正文".to_string(),
        word_count: 2,
        status: "draft".to_string(),
        summary: summary.map(str::to_string),
        style_tags: None,
        analysis_data,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn layered_chapter_summary_lines_omits_missing_summary_without_placeholders() {
    let chapters = vec![
        chapter_for_context(1, Some("已有标题"), None, None),
        chapter_for_context(2, None, Some("有效摘要。后续句子"), None),
    ];

    let text = layered_chapter_summary_lines(&chapters, 3).join("\n");

    assert!(!text.contains("无标题"));
    assert!(!text.contains("无摘要"));
    assert!(!text.contains("已有标题"));
    assert!(text.contains("第2章：有效摘要"));
}

#[test]
fn chapter_analysis_context_line_omits_missing_title_without_placeholder() {
    let chapter = chapter_for_context(
        7,
        None,
        None,
        Some(serde_json::json!({
            "key_events": [{ "description": "主角进入边境城" }]
        })),
    );

    let line = chapter_analysis_context_line(&chapter).expect("analysis context line should exist");

    assert!(line.starts_with("第7章："));
    assert!(!line.contains("无标题"));
    assert!(line.contains("主角进入边境城"));
}

fn profile_for_context(last_synced_chapter: i32) -> CharacterProfileRecord {
    let now = chrono::Utc::now().naive_utc();
    CharacterProfileRecord {
        id: 1,
        project_id: 1,
        name: "林岚".to_string(),
        gender: None,
        age: None,
        base_attributes: serde_json::json!({}),
        current_state: serde_json::json!({
            "explicit_attributes": {
                "appearance": "穿青色劲装，左腕有旧伤",
                "weapon": "短刀"
            },
            "implicit_attributes": { "personality": "谨慎但会为弱者冒险" },
            "social_attributes": { "身份": "边城医馆学徒" }
        }),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn character_state_context_line_includes_profile_constraints() {
    let now = chrono::Utc::now().naive_utc();
    let state_index = CharacterStateIndexRecord {
        id: 1,
        profile_id: 1,
        chapter_number: 3,
        state_summary: "刚逃出城门，正在寻找线索".to_string(),
        key_attributes: serde_json::json!({
            "explicit_attributes": { "appearance": "衣摆沾血", "weapon": "短刀" },
            "implicit_attributes": { "personality": "谨慎" },
            "social_attributes": {}
        }),
        emotional_state: Some("警惕".to_string()),
        current_location: Some("北城门".to_string()),
        current_goal: Some("找到失踪的商队".to_string()),
        created_at: now,
    };
    let profile = profile_for_context(3);

    let line = character_state_context_line(&state_index, &profile)
        .expect("state context should be rendered");

    assert!(line.contains("第3章后，林岚"));
    assert!(line.contains("显性特征: appearance: 衣摆沾血"));
    assert!(line.contains("性格特质: personality: 谨慎"));
    assert!(line.contains("目标: 找到失踪的商队"));
}

#[test]
fn profile_context_line_skips_future_synced_state() {
    let profile = profile_for_context(8);

    assert!(character_profile_context_line(&profile, 8).is_none());
    assert!(character_profile_context_line(&profile, 7).is_none());
    assert!(
        character_profile_context_line(&profile, 9)
            .expect("past state should be usable")
            .contains("第8章后，林岚")
    );
}

use super::super::*;
use a4n_core::models::project::text_anchor;
use a4n_services::chapter::source_spans::source_spans_for_chapters;
use a4n_services::text_anchor::text_anchor_prompt_values;

pub(super) fn discovery_chapter(
    chapter_number: i32,
    status: &str,
    content: &str,
) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: i64::from(chapter_number),
        project_id: 1,
        outline_id: None,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

pub(super) fn canonical_character_sync_value() -> Value {
    json!({
        "attribute_changes": [{
            "chapter_number": 1,
            "category": "explicit",
            "key": "personality",
            "operation": "modify",
            "new_value": "冷静",
            "change_description": "表现更冷静",
            "event_cause": "面对线索时压住情绪",
            "evidence_anchor_id": 1
        }],
        "behaviors": [{
            "chapter_number": 1,
            "behavior_type": "decision",
            "behavior_description": "谨慎分析线索",
            "original_text_anchor_id": 1,
            "behavior_tags": ["谨慎"],
            "related_characters": [],
            "impact_analysis": "强化冷静特质"
        }],
        "deletion_candidates": [],
        "chapter_states": [{
            "chapter_number": 1,
            "state_summary": "冷静寻找线索",
            "current_location": null,
            "current_goal": "寻找线索",
            "emotional_state": "紧张"
        }]
    })
}

pub(super) fn character_sync_context_for_test(name: &str) -> CharacterSyncContext {
    let now = chrono::Utc::now().naive_utc();
    let content = "他沉默片刻后才开口";
    let chapter = discovery_chapter(1, "completed", content);
    let anchors = text_anchors_for_test(std::slice::from_ref(&chapter));
    CharacterSyncContext {
        profile: protagonist_profile::Model {
            id: 1,
            project_id: 1,
            name: name.to_string(),
            gender: None,
            age: None,
            base_attributes: json!({}),
            current_state: json!({
                "explicit_attributes": {
                    "personality": "谨慎"
                },
                "implicit_attributes": {},
                "social_attributes": {}
            }),
            character_arc: None,
            voice_fingerprint: None,
            appearance: None,
            background: None,
            personality: None,
            last_synced_chapter: 0,
            created_at: now,
            updated_at: now,
        },
        start_chapter: 1,
        target_chapter: 1,
        clear_to_chapter: 1,
        selected_chapter_numbers: vec![1],
        chapter_data: vec![json!({
            "chapter_number": 1,
            "title": "第一章",
            "content": content,
            "text_anchors": text_anchor_prompt_values(&[chapter], &anchors)
        })],
        baseline_state: json!({
            "explicit_attributes": {
                "personality": "谨慎"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
    }
}

pub(super) fn text_anchors_for_test(chapters: &[chapter::Model]) -> Vec<text_anchor::Model> {
    let now = chrono::Utc::now().naive_utc();
    source_spans_for_chapters(chapters)
        .iter()
        .enumerate()
        .map(|(index, span)| text_anchor::Model {
            id: i64::try_from(index + 1).unwrap(),
            project_id: 1,
            source_chapter_id: Some(span.chapter_id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_kind: "chapter_content".to_string(),
            source_id: span.chapter_id,
            paragraph_index: i32::try_from(span.sentence_index).unwrap(),
            start_offset: i32::try_from(span.char_start).unwrap(),
            end_offset: i32::try_from(span.char_end).unwrap(),
            quote: span.text.clone(),
            quote_hash: format!("hash-{}", index + 1),
            source_version_hash: "source-version".to_string(),
            created_at: now,
        })
        .collect()
}

pub(super) fn parse_sync_value(value: Value) -> Result<Value, String> {
    parse_character_sync_output(&serde_json::to_string(&value).expect("json should serialize"))
}

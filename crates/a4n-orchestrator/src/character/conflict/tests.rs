use serde_json::json;

use crate::character::conflict_context_rules::CharacterConflictChapterScope;

use super::{
    contracts::CharacterConflictCheckInput,
    execution::required_character_conflict_input,
    request::{
        BuildCharacterConflictRequestInput, PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT,
        build_character_conflict_request, character_conflict_call_key,
    },
    workflow::character_conflict_workflow_run_input,
};

#[test]
fn required_input_normalizes_name_and_validates_scope() {
    let required = required_character_conflict_input(&CharacterConflictCheckInput {
        project_id: 9,
        name: " 陈恪 ".to_string(),
        start_chapter: Some(2),
        end_chapter: Some(5),
    })
    .expect("valid character conflict input");
    assert_eq!(required.name, "陈恪");
    assert_eq!(required.chapter_scope.start_chapter, Some(2));
    assert_eq!(required.chapter_scope.end_chapter, Some(5));

    assert!(
        required_character_conflict_input(&CharacterConflictCheckInput {
            project_id: 0,
            name: "陈恪".to_string(),
            start_chapter: None,
            end_chapter: None,
        })
        .is_err()
    );
    assert!(
        required_character_conflict_input(&CharacterConflictCheckInput {
            project_id: 9,
            name: "  ".to_string(),
            start_chapter: None,
            end_chapter: None,
        })
        .is_err()
    );
    assert!(
        required_character_conflict_input(&CharacterConflictCheckInput {
            project_id: 9,
            name: "陈恪".to_string(),
            start_chapter: Some(5),
            end_chapter: Some(2),
        })
        .is_err()
    );
}

#[test]
fn request_uses_stable_context_manifest_and_json_contract() {
    let scope = CharacterConflictChapterScope::new(Some(2), Some(5)).unwrap();
    let role_attr = json!({"source_id": "profile", "base_attributes": {"goal": "留城"}});
    let chapters = vec![json!({"source_id": "chapter:3", "summary": "主角已经离城"})];
    let states = vec![json!({"source_id": "state:3", "current_goal": "离城"})];
    let events = vec![json!({"source_id": "attribute_event:7", "key": "goal"})];
    let call_key = character_conflict_call_key(17, "陈恪");
    let request = build_character_conflict_request(BuildCharacterConflictRequestInput {
        system_prompt: "system".to_string(),
        project_id: 9,
        profile_id: 17,
        profile_name: "陈恪",
        chapter_scope: scope,
        role_attr: &role_attr,
        chapter_context: &chapters,
        state_timeline: &states,
        attribute_events: &events,
        call_key: call_key.clone(),
    })
    .expect("request should build");

    assert_eq!(request.max_tokens_profile.as_deref(), Some("json_standard"));
    assert_eq!(
        request
            .trace_context
            .as_ref()
            .and_then(|context| context.call_key.as_deref()),
        Some(call_key.as_str())
    );
    let manifest = request
        .context_manifest_json
        .expect("context-aware request must carry manifest");
    assert_eq!(
        manifest["packs"][0]["pack_kind"],
        PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT
    );
    let manifest_text = serde_json::to_string(&manifest).unwrap();
    assert!(manifest_text.contains("chapter:3"));
    assert!(manifest_text.contains("state:3"));
    assert!(manifest_text.contains("attribute_event:7"));
    assert!(
        request
            .messages
            .last()
            .unwrap()
            .content
            .contains("severity_values")
    );
}

#[test]
fn workflow_input_uses_canonical_fingerprint_and_call_key() {
    let scope = CharacterConflictChapterScope::new(None, Some(8)).unwrap();
    let request = build_character_conflict_request(BuildCharacterConflictRequestInput {
        system_prompt: "system".to_string(),
        project_id: 9,
        profile_id: 17,
        profile_name: "陈恪",
        chapter_scope: scope,
        role_attr: &json!({"source_id": "profile"}),
        chapter_context: &[json!({"source_id": "chapter:3", "summary": "摘要"})],
        state_timeline: &[],
        attribute_events: &[],
        call_key: "character_conflict:17:陈恪".to_string(),
    })
    .unwrap();
    let (input, fingerprint) = character_conflict_workflow_run_input(
        9,
        "陈恪",
        scope,
        "character_conflict:17:陈恪",
        &request,
        Some("character-conflict-task"),
    )
    .expect("workflow input should build");

    assert_eq!(input.workflow_kind, "character_conflict_check");
    assert_eq!(
        input.current_step_key.as_deref(),
        Some("character_conflict:17:陈恪")
    );
    assert_eq!(fingerprint["project_id"], json!(9));
    assert_eq!(fingerprint["character_name"], json!("陈恪"));
    assert_eq!(fingerprint["chapter_scope"]["end_chapter"], json!(8));
    assert_eq!(
        input.active_task_id.as_deref(),
        Some("character-conflict-task")
    );
}

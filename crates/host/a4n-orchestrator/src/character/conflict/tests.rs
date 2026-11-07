use a4n_api_contract::character::CharacterConflictCheckInput;
use a4n_llm_context::preview_context_aware_chat_request;
use serde_json::json;

use crate::character::conflict::context_rules::CharacterConflictChapterScope;

use super::{
    execution::validated_character_conflict_input,
    request::{
        BuildCharacterConflictRequestInput, PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT,
        build_character_conflict_request, character_conflict_call_key,
    },
    workflow::character_conflict_workflow_run_input,
};

#[test]
fn input_contract_normalizes_name_and_validates_scope() {
    let validated = validated_character_conflict_input(&CharacterConflictCheckInput {
        project_id: 9,
        name: " 陈恪 ".to_string(),
        start_chapter: Some(2),
        end_chapter: Some(5),
    })
    .expect("valid character conflict input");
    assert_eq!(validated.name, "陈恪");
    assert_eq!(validated.chapter_scope.start_chapter, Some(2));
    assert_eq!(validated.chapter_scope.end_chapter, Some(5));

    assert!(
        validated_character_conflict_input(&CharacterConflictCheckInput {
            project_id: 0,
            name: "陈恪".to_string(),
            start_chapter: None,
            end_chapter: None,
        })
        .is_err()
    );
    assert!(
        validated_character_conflict_input(&CharacterConflictCheckInput {
            project_id: 9,
            name: "  ".to_string(),
            start_chapter: None,
            end_chapter: None,
        })
        .is_err()
    );
    assert!(
        validated_character_conflict_input(&CharacterConflictCheckInput {
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
    let expected_call_key = character_conflict_call_key(17);
    let (call_key, mut request) =
        build_character_conflict_request(BuildCharacterConflictRequestInput {
            system_prompt: "system".to_string(),
            project_id: 9,
            profile_id: 17,
            profile_name: "陈恪",
            chapter_scope: scope,
            role_attr: &role_attr,
            chapter_context: &chapters,
            state_timeline: &states,
            attribute_events: &events,
        })
        .expect("request should build");

    assert_eq!(call_key, expected_call_key);
    assert!(request.max_tokens.is_none());
    assert_eq!(
        request
            .trace_context
            .as_ref()
            .and_then(|context| context.call_key.as_deref()),
        Some(call_key.as_str())
    );
    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context-aware request must carry manifest");
    assert_eq!(
        manifest["packs"][0]["pack_kind"],
        PACK_KIND_CHARACTER_CONFLICT_SOURCE_CONTEXT
    );
    let manifest_text = serde_json::to_string(&manifest).unwrap();
    assert!(manifest_text.contains("chapter:3"));
    assert!(manifest_text.contains("state:3"));
    assert!(manifest_text.contains("attribute_event:7"));
    preview_context_aware_chat_request(&mut request, 1_000_000, 8192)
        .expect("render the same context sent to the provider");
    let sent = &request.messages.last().unwrap().content;
    assert!(
        sent.contains("陈恪"),
        "canonical identity must reach the model"
    );
    assert!(sent.contains("character_name"));
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
fn request_fingerprint_tracks_canonical_character_name() {
    let build = |name| {
        build_character_conflict_request(BuildCharacterConflictRequestInput {
            system_prompt: "system".to_string(),
            project_id: 9,
            profile_id: 17,
            profile_name: name,
            chapter_scope: CharacterConflictChapterScope::new(None, None).unwrap(),
            role_attr: &json!({"source_id": "profile"}),
            chapter_context: &[],
            state_timeline: &[],
            attribute_events: &[],
        })
        .unwrap()
        .1
        .trace_context
        .unwrap()
        .input_fingerprint_sha256
    };
    assert_ne!(build("陈恪"), build("林晓雨"));
}

#[test]
fn workflow_input_uses_canonical_fingerprint_and_call_key() {
    let scope = CharacterConflictChapterScope::new(None, Some(8)).unwrap();
    let (call_key, request) =
        build_character_conflict_request(BuildCharacterConflictRequestInput {
            system_prompt: "system".to_string(),
            project_id: 9,
            profile_id: 17,
            profile_name: "陈恪",
            chapter_scope: scope,
            role_attr: &json!({"source_id": "profile"}),
            chapter_context: &[json!({"source_id": "chapter:3", "summary": "摘要"})],
            state_timeline: &[],
            attribute_events: &[],
        })
        .unwrap();
    let (input, fingerprint) = character_conflict_workflow_run_input(
        9,
        17,
        "陈恪",
        scope,
        &call_key,
        &request,
        Some("character-conflict-task"),
    )
    .expect("workflow input should build");

    assert_eq!(input.workflow_kind, "character_conflict_check");
    assert_eq!(input.current_step_key.as_deref(), Some(call_key.as_str()));
    assert_eq!(input.scope_id, Some(17));
    assert_eq!(fingerprint["project_id"], json!(9));
    assert_eq!(fingerprint["character_name"], json!("陈恪"));
    assert_eq!(fingerprint["chapter_scope"]["end_chapter"], json!(8));
    assert_eq!(
        input.active_task_id.as_deref(),
        Some("character-conflict-task")
    );
}

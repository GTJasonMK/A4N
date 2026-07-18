use super::snapshots::character_sync_required_i32;
use super::validation::validate_character_sync_persistence_input;
use super::{
    CharacterSyncChapterStateRecordInput, CharacterSyncPersistenceInput,
    apply_character_sync_persistence_record,
};
use crate::character::CharacterDbError;
use sea_orm::DatabaseConnection;
use serde_json::json;

fn valid_input() -> CharacterSyncPersistenceInput {
    CharacterSyncPersistenceInput {
        project_id: 3,
        profile_id: 7,
        profile_name: "陈恪".to_string(),
        start_chapter: 1,
        target_chapter: 1,
        clear_to_chapter: Some(1),
        selected_chapter_numbers: vec![1],
        baseline_state: json!({
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {}
        }),
        write_steps: vec![],
        behaviors: vec![],
        classification_suggestions: vec![],
        deletion_candidates: vec![],
        chapter_states: vec![CharacterSyncChapterStateRecordInput {
            chapter_number: 1,
            state_summary: "初始状态".to_string(),
            current_location: None,
            current_goal: None,
            emotional_state: None,
        }],
        implicit_review_decisions: json!([]),
        generation_run: crate::generation::RecordCompletedGenerationRunInput {
            project_id: 3,
            workflow: "character_sync".to_string(),
            step_key: "persist".to_string(),
            title: "角色同步".to_string(),
            model_kind: "llm".to_string(),
            provider: None,
            model: None,
            prompt_template_names_json: None,
            prompt_template_hashes_json: None,
            rendered_messages_json: None,
            rendered_prompts_json: None,
            input_variables_json: None,
            source_refs_json: None,
            media_refs_json: None,
            request_json: None,
            response_summary_json: None,
            targets: vec![],
        },
    }
}

#[test]
fn sync_json_field_contract_preserves_typed_error() {
    let error = character_sync_required_i32(&json!({}), "chapter_number")
        .expect_err("missing chapter field must fail");
    assert!(matches!(
        error,
        CharacterDbError::InvalidArgument(message) if message == "缺少字段 chapter_number"
    ));
}

#[test]
fn sync_input_contract_rejects_invalid_shapes_before_database_access() {
    let mut input = valid_input();
    input.selected_chapter_numbers.clear();
    assert!(matches!(
        validate_character_sync_persistence_input(&input),
        Err(CharacterDbError::InvalidArgument(message)) if message == "角色同步缺少选中章节"
    ));

    let mut input = valid_input();
    input.target_chapter = 2;
    assert!(matches!(
        validate_character_sync_persistence_input(&input),
        Err(CharacterDbError::InvalidArgument(message))
            if message == "角色同步选中章节缺少目标章节: 2"
    ));

    let mut input = valid_input();
    input.implicit_review_decisions = json!({});
    assert!(matches!(
        validate_character_sync_persistence_input(&input),
        Err(CharacterDbError::InvalidArgument(message))
            if message == "角色同步隐性复审决策快照必须是 JSON 数组"
    ));
}

#[tokio::test]
async fn sync_persistence_preserves_database_error() {
    let disconnected = DatabaseConnection::default();
    let error = apply_character_sync_persistence_record(&disconnected, valid_input())
        .await
        .expect_err("transaction begin on a disconnected database must fail");
    assert!(matches!(error, CharacterDbError::Database(_)));
}

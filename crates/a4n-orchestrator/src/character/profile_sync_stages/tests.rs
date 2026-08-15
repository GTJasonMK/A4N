use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use a4n_core::{error::AppError, models::generation::workflow_checkpoint};
use a4n_db::{
    DatabaseConnection,
    workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING, upsert_workflow_run},
};
use a4n_llm::provider::{ChatMessage, ChatResponse};
use a4n_prompt::rendering::PromptRenderer;
use a4n_test_support::llm::{CapturingSequenceLlmProvider, CountingLlmProvider};
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};

use super::{
    contracts::{
        CharacterProfileSyncStage, CharacterProfileSyncStageInput,
        CharacterProfileSyncStageTraceInput, CharacterProfileSyncStageTracePort,
    },
    output::{
        parse_character_profile_sync_stage_output, validate_character_profile_sync_extraction,
    },
    request::{
        CHARACTER_PROFILE_SYNC_SESSION_PROMPT, build_character_profile_sync_stage_request,
        character_profile_sync_stage_contract,
    },
};
use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

static NEXT_WORKFLOW_KEY: AtomicUsize = AtomicUsize::new(1);

struct TestPromptRenderer;

#[async_trait]
impl PromptRenderer for TestPromptRenderer {
    async fn render_prompt(
        &self,
        template_name: &str,
        _variables: &HashMap<String, Value>,
        _project_id: Option<i64>,
    ) -> Result<String, AppError> {
        assert!(
            template_name == CHARACTER_PROFILE_SYNC_SESSION_PROMPT
                || CharacterProfileSyncStage::ALL
                    .iter()
                    .any(|stage| stage.prompt_name() == template_name)
        );
        Ok(format!("system prompt for {template_name}"))
    }
}

#[derive(Default)]
struct TestTracePort {
    phases: Mutex<Vec<String>>,
}

impl CharacterProfileSyncStageTracePort for TestTracePort {
    fn record_stage_trace(&self, input: CharacterProfileSyncStageTraceInput) -> Option<String> {
        self.phases
            .lock()
            .expect("trace phases lock should not be poisoned")
            .push(input.phase.clone());
        Some(format!("/tmp/{}.json", input.phase))
    }
}

async fn setup_context(
    llm: Arc<dyn a4n_llm::provider::LlmProvider>,
) -> (Arc<DatabaseConnection>, OrchestratorContext) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .with_llm(llm)
        .build()
        .await;
    (database, context)
}

async fn create_workflow(database: &DatabaseConnection) -> i64 {
    let sequence = NEXT_WORKFLOW_KEY.fetch_add(1, Ordering::SeqCst);
    upsert_workflow_run(
        database,
        UpsertWorkflowRunInput {
            business_key: format!("profile-sync-stages-test-{}-{sequence}", std::process::id()),
            workflow_kind: "character_profile_sync".to_string(),
            project_id: None,
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: None,
            input_fingerprint_json: Some(json!({"sequence": sequence})),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: None,
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
    )
    .await
    .expect("workflow should be created")
    .id
}

fn stage_input(workflow_run_id: Option<i64>) -> CharacterProfileSyncStageInput {
    CharacterProfileSyncStageInput {
        project_id: 1,
        workflow_run_id,
        profile_id: 7,
        profile_name: "林夏".to_string(),
        last_synced_chapter: 0,
        start_chapter: 1,
        target_chapter: 1,
        selected_chapter_numbers: vec![1],
        chapter_data: vec![json!({
            "chapter_number": 1,
            "content": "林夏停在巷口观察后，快步挡在少女身前。",
            "text_anchors": [{
                "id": 11,
                "quote": "林夏停在巷口观察后，快步挡在少女身前。"
            }]
        })],
        baseline_state: json!({
            "explicit_attributes": {"位置": "城门"},
            "implicit_attributes": {"谨慎": "行动前先观察"},
            "social_attributes": {}
        }),
        base_attributes: json!({
            "explicit_attributes": {"位置": "城门"},
            "implicit_attributes": {"谨慎": "行动前先观察"},
            "social_attributes": {}
        }),
    }
}

fn canonical_stage_responses() -> Vec<String> {
    vec![
        json!({
            "events": [{
                "chapter_number": 1,
                "event_type": "action",
                "summary": "观察环境后保护少女",
                "evidence_anchor_id": 11,
                "related_characters": ["少女"]
            }]
        })
        .to_string(),
        json!({
            "attribute_changes": [{
                "chapter_number": 1,
                "category": "explicit",
                "key": "位置",
                "operation": "modify",
                "new_value": "巷口",
                "change_description": "角色移动到巷口",
                "event_cause": "保护少女",
                "evidence_anchor_id": 11
            }]
        })
        .to_string(),
        json!({
            "chapter_states": [{
                "chapter_number": 1,
                "state_summary": "在巷口保护少女",
                "current_location": "巷口",
                "current_goal": "确保少女安全",
                "emotional_state": "警觉"
            }]
        })
        .to_string(),
        json!({
            "behaviors": [{
                "chapter_number": 1,
                "behavior_type": "action",
                "behavior_description": "观察后保护少女",
                "original_text_anchor_id": 11,
                "behavior_tags": ["谨慎", "保护"],
                "related_characters": ["少女"],
                "impact_analysis": "改变少女处境"
            }]
        })
        .to_string(),
        json!({"deletion_candidates": []}).to_string(),
    ]
}

fn canonical_attribute_change() -> Value {
    json!({
        "chapter_number": 1,
        "category": "explicit",
        "key": "位置",
        "operation": "modify",
        "new_value": "巷口",
        "change_description": "角色移动到巷口",
        "event_cause": "保护少女",
        "evidence_anchor_id": 11
    })
}

fn canonical_behavior() -> Value {
    json!({
        "chapter_number": 1,
        "behavior_type": "action",
        "behavior_description": "观察后保护少女",
        "original_text_anchor_id": 11,
        "behavior_tags": ["谨慎", "保护"],
        "related_characters": ["少女"],
        "impact_analysis": "改变少女处境"
    })
}

fn canonical_deletion_candidate() -> Value {
    json!({
        "chapter_number": 1,
        "category": "explicit",
        "key": "旧伤",
        "reason": "正文明确说明伤势痊愈",
        "evidence_anchor_id": 11
    })
}

fn canonical_chapter_state(chapter_number: i32) -> Value {
    json!({
        "chapter_number": chapter_number,
        "state_summary": "在巷口保护少女",
        "current_location": "巷口",
        "current_goal": "确保少女安全",
        "emotional_state": "警觉"
    })
}

fn parse_stage_value(
    stage: CharacterProfileSyncStage,
    value: Value,
    input: &CharacterProfileSyncStageInput,
) -> Result<Value, String> {
    parse_character_profile_sync_stage_output(
        stage,
        &value.to_string(),
        input,
        &mut super::contracts::CharacterProfileSyncStageOutputs::default(),
    )
}

fn assert_stage_error_contains(
    stage: CharacterProfileSyncStage,
    value: Value,
    input: &CharacterProfileSyncStageInput,
    expected: &str,
) {
    let error = parse_stage_value(stage, value, input).expect_err("invalid stage output must fail");
    assert!(
        error.contains(expected),
        "expected error containing {expected:?}, got {error:?}"
    );
}

#[test]
fn contracts_make_canonical_category_values_explicit() {
    for stage in [
        CharacterProfileSyncStage::AttributeChanges,
        CharacterProfileSyncStage::DeletionCandidates,
    ] {
        let contract = character_profile_sync_stage_contract(stage);
        assert_eq!(
            contract["field_value_rules"]["category"]["allowed_values"],
            json!(["explicit", "implicit", "social"])
        );
        assert_eq!(
            contract["field_value_rules"]["category"]["forbidden_values"],
            json!([
                "explicit_attributes",
                "implicit_attributes",
                "social_attributes"
            ])
        );
    }
}

#[tokio::test]
async fn stage_requests_share_cache_prefix_only_within_one_character_session() {
    let input = stage_input(None);
    let evidence = build_character_profile_sync_stage_request(
        &input,
        CharacterProfileSyncStage::EvidenceEvents,
        &json!({}),
        &TestPromptRenderer,
    )
    .await
    .expect("evidence request");
    let attributes = build_character_profile_sync_stage_request(
        &input,
        CharacterProfileSyncStage::AttributeChanges,
        &json!({"character_sync_evidence_events": {"events": []}}),
        &TestPromptRenderer,
    )
    .await
    .expect("attribute request");

    assert_eq!(evidence.messages[0].content, attributes.messages[0].content);
    assert!(
        evidence.messages[1]
            .content
            .contains("system prompt for character_sync_evidence_events")
    );
    assert!(
        attributes.messages[1]
            .content
            .contains("system prompt for character_sync_attribute_changes")
    );
    let evidence_pack = &evidence.context_manifest_json.as_ref().unwrap()["packs"][0];
    let attributes_pack = &attributes.context_manifest_json.as_ref().unwrap()["packs"][0];
    assert_eq!(evidence_pack["config_hash"], attributes_pack["config_hash"]);
    assert_eq!(evidence_pack["variants"], attributes_pack["variants"]);

    let mut other_character = input;
    other_character.profile_id = 8;
    other_character.profile_name = "另一角色".to_string();
    let other = build_character_profile_sync_stage_request(
        &other_character,
        CharacterProfileSyncStage::EvidenceEvents,
        &json!({}),
        &TestPromptRenderer,
    )
    .await
    .expect("other character request");
    let other_pack = &other.context_manifest_json.as_ref().unwrap()["packs"][0];
    assert_ne!(evidence_pack["variants"], other_pack["variants"]);
}

#[tokio::test]
async fn preview_only_prepares_the_real_initial_request() {
    let provider = Arc::new(CapturingSequenceLlmProvider::new_owned(Vec::new()));
    let (_database, context) = setup_context(provider).await;
    let step = context
        .prepare_character_profile_sync_initial_request_preview_validated(
            stage_input(None),
            &TestPromptRenderer,
        )
        .await
        .expect("stage preview should succeed");

    assert_eq!(step.prompt_name, "character_sync_evidence_events");
    assert_eq!(
        step.request.trace_context.unwrap().step_key,
        "character_sync_evidence_events"
    );
}

#[tokio::test]
async fn application_persists_restores_and_revalidates_all_stage_checkpoints() {
    let provider = Arc::new(CapturingSequenceLlmProvider::new_owned(
        canonical_stage_responses(),
    ));
    let (database, context) = setup_context(provider.clone()).await;
    let workflow_run_id = create_workflow(&database).await;
    let trace_port = TestTracePort::default();
    let input = stage_input(Some(workflow_run_id));

    let output = context
        .run_character_profile_sync_stages_validated(
            input.clone(),
            &TestPromptRenderer,
            None,
            Some(&trace_port),
        )
        .await
        .expect("staged sync application should succeed");
    assert_eq!(provider.captured_user_messages().len(), 5);
    assert_eq!(
        output.extraction.changes[0]["evidence"],
        json!("林夏停在巷口观察后，快步挡在少女身前。")
    );
    assert!(
        output.extraction.changes[0]
            .get("evidence_anchor_id")
            .is_none()
    );
    assert_eq!(
        output.extraction.behaviors[0]["original_text"],
        json!("林夏停在巷口观察后，快步挡在少女身前。")
    );
    assert_eq!(
        output.extraction.chapter_states[0]
            .current_location
            .as_deref(),
        Some("巷口")
    );
    assert_eq!(output.llm_trace_paths.len(), 5);
    assert_eq!(
        output.generation_trace.response_chars,
        canonical_stage_responses()
            .iter()
            .map(|value| value.chars().count())
            .sum::<usize>()
    );

    let checkpoints = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .all(database.as_ref())
        .await
        .expect("checkpoint query should succeed");
    assert_eq!(checkpoints.len(), 5);

    let restored = context
        .run_character_profile_sync_stages_validated(input.clone(), &TestPromptRenderer, None, None)
        .await
        .expect("matching checkpoints should restore");
    assert_eq!(restored.extraction, output.extraction);
    assert_eq!(provider.captured_user_messages().len(), 5);

    let first = checkpoints
        .into_iter()
        .find(|checkpoint| {
            checkpoint
                .step_key
                .ends_with("character_sync_evidence_events")
        })
        .expect("evidence checkpoint should exist");
    let response_content = first
        .checkpoint_json
        .as_ref()
        .expect("checkpoint json should exist")["response_content"]
        .clone();
    let mut active: workflow_checkpoint::ActiveModel = first.into();
    active.checkpoint_json = Set(Some(json!({
        "output": {"events": []},
        "parsed_output": {"events": []},
        "response_content": response_content,
        "usage": null,
        "finish_reason": "stop"
    })));
    active
        .update(database.as_ref())
        .await
        .expect("checkpoint tamper setup should succeed");

    let error = context
        .run_character_profile_sync_stages_validated(input, &TestPromptRenderer, None, None)
        .await
        .expect_err("mismatched parsed output must fail");
    assert!(
        error
            .to_string()
            .contains("parsed_output 与 response_content 重新解析结果不一致")
    );
    assert_eq!(provider.captured_user_messages().len(), 5);
}

#[tokio::test]
async fn application_rejects_truncated_response_without_checkpoint() {
    let provider = Arc::new(CountingLlmProvider::new(ChatResponse {
        message: ChatMessage::assistant(canonical_stage_responses()[0].clone()),
        usage: None,
        finish_reason: Some("length".to_string()),
    }));
    let (database, context) = setup_context(provider.clone()).await;
    let workflow_run_id = create_workflow(&database).await;

    let error = context
        .run_character_profile_sync_stages_validated(
            stage_input(Some(workflow_run_id)),
            &TestPromptRenderer,
            None,
            None,
        )
        .await
        .expect_err("truncated stage response must fail");
    assert!(matches!(&error, AppError::InvalidArgument(_)));
    assert!(error.to_string().contains("模型输出被截断"));
    assert_eq!(provider.stream_call_count(), 1);
    let checkpoints = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .all(database.as_ref())
        .await
        .expect("checkpoint query should succeed");
    assert!(checkpoints.is_empty());
}

#[test]
fn output_rejects_unknown_fields_and_invalid_anchor_contracts() {
    let input = stage_input(Some(1));
    let mut outputs = super::contracts::CharacterProfileSyncStageOutputs::default();
    let unknown = parse_character_profile_sync_stage_output(
        CharacterProfileSyncStage::EvidenceEvents,
        r#"{"events":[],"notes":"extra"}"#,
        &input,
        &mut outputs,
    )
    .expect_err("unknown root field should fail");
    assert!(unknown.contains("未支持字段 `notes`"));

    let legacy = parse_character_profile_sync_stage_output(
        CharacterProfileSyncStage::AttributeChanges,
        &json!({
            "attribute_changes": [{
                "chapter_number": 1,
                "category": "explicit",
                "key": "位置",
                "operation": "modify",
                "new_value": "巷口",
                "change_description": "移动",
                "event_cause": "保护少女",
                "evidence": "自由文本"
            }]
        })
        .to_string(),
        &input,
        &mut outputs,
    )
    .expect_err("legacy evidence should fail");
    assert!(legacy.contains("evidence_anchor_id"));
}

#[test]
fn extraction_materializes_anchor_evidence_and_validates_projection() {
    let input = stage_input(Some(1));
    let extraction = validate_character_profile_sync_extraction(
        super::contracts::CharacterProfileSyncExtraction {
            changes: vec![json!({
                "chapter_number": 1,
                "category": "explicit",
                "key": "位置",
                "operation": "modify",
                "new_value": "巷口",
                "change_description": "移动",
                "event_cause": "保护少女",
                "evidence_anchor_id": 11
            })],
            behaviors: Vec::new(),
            deletion_candidates: Vec::new(),
            chapter_states: Vec::new(),
        },
        &input,
    )
    .expect("canonical extraction should materialize evidence");
    assert_eq!(
        extraction.changes[0]["evidence"],
        json!("林夏停在巷口观察后，快步挡在少女身前。")
    );
}

#[test]
fn attribute_change_contract_rejects_system_owned_and_invalid_fields() {
    let input = stage_input(Some(1));

    let mut direct_delete = canonical_attribute_change();
    direct_delete["operation"] = json!("delete");
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [direct_delete]}),
        &input,
        "deletion_candidates",
    );

    let mut old_value = canonical_attribute_change();
    old_value["old_value"] = json!("城门");
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [old_value]}),
        &input,
        "old_value 由系统计算",
    );

    let mut null_new_value = canonical_attribute_change();
    null_new_value["new_value"] = Value::Null;
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [null_new_value]}),
        &input,
        "new_value 不能为空",
    );

    let mut non_numeric_anchor = canonical_attribute_change();
    non_numeric_anchor["evidence_anchor_id"] = json!("chapter-1-anchor-11");
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [non_numeric_anchor]}),
        &input,
        "JSON 整数数字",
    );

    let mut non_positive_anchor = canonical_attribute_change();
    non_positive_anchor["evidence_anchor_id"] = json!(0);
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [non_positive_anchor]}),
        &input,
        "必须大于 0",
    );

    let mut legacy_anchor = canonical_attribute_change();
    legacy_anchor
        .as_object_mut()
        .expect("attribute change should be object")
        .remove("evidence_anchor_id");
    legacy_anchor["evidence_span_id"] = json!(11);
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [legacy_anchor]}),
        &input,
        "evidence_span_id 已废弃",
    );
}

#[test]
fn behavior_contract_rejects_non_strings_legacy_fields_and_inline_classification() {
    let input = stage_input(Some(1));

    let mut tags = canonical_behavior();
    tags["behavior_tags"] = json!(["谨慎", 7]);
    assert_stage_error_contains(
        CharacterProfileSyncStage::BehaviorRecords,
        json!({"behaviors": [tags]}),
        &input,
        "behavior_tags[1] 必须是字符串",
    );

    let mut related = canonical_behavior();
    related["related_characters"] = json!(["少女", {"name": "沈砚"}]);
    assert_stage_error_contains(
        CharacterProfileSyncStage::BehaviorRecords,
        json!({"behaviors": [related]}),
        &input,
        "related_characters[1] 必须是字符串",
    );

    let mut legacy_anchor = canonical_behavior();
    legacy_anchor
        .as_object_mut()
        .expect("behavior should be object")
        .remove("original_text_anchor_id");
    legacy_anchor["original_text_span_id"] = json!(11);
    assert_stage_error_contains(
        CharacterProfileSyncStage::BehaviorRecords,
        json!({"behaviors": [legacy_anchor]}),
        &input,
        "original_text_span_id 已废弃",
    );

    let mut classified = canonical_behavior();
    classified["classification_results"] = json!({"谨慎": "conform"});
    assert_stage_error_contains(
        CharacterProfileSyncStage::BehaviorRecords,
        json!({"behaviors": [classified]}),
        &input,
        "不应包含 classification_results",
    );
}

#[test]
fn deletion_and_chapter_state_contracts_reject_duplicates_gaps_and_range_errors() {
    let input = stage_input(Some(1));
    let candidate = canonical_deletion_candidate();
    assert_stage_error_contains(
        CharacterProfileSyncStage::DeletionCandidates,
        json!({"deletion_candidates": [candidate.clone(), candidate]}),
        &input,
        "删除候选重复",
    );

    let mut two_chapter_input = stage_input(Some(1));
    two_chapter_input.target_chapter = 2;
    two_chapter_input.selected_chapter_numbers = vec![1, 2];
    assert_stage_error_contains(
        CharacterProfileSyncStage::ChapterStates,
        json!({"chapter_states": [canonical_chapter_state(1)]}),
        &two_chapter_input,
        "缺少章节状态",
    );
    assert_stage_error_contains(
        CharacterProfileSyncStage::ChapterStates,
        json!({"chapter_states": [canonical_chapter_state(1), canonical_chapter_state(1)]}),
        &two_chapter_input,
        "重复章节",
    );
    assert_stage_error_contains(
        CharacterProfileSyncStage::ChapterStates,
        json!({"chapter_states": [canonical_chapter_state(1), canonical_chapter_state(3)]}),
        &two_chapter_input,
        "超出本次同步章节",
    );

    let mut out_of_range = canonical_attribute_change();
    out_of_range["chapter_number"] = json!(3);
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [out_of_range]}),
        &two_chapter_input,
        "不在本次同步输入章节中",
    );
}

#[test]
fn stage_contract_rejects_non_json_objects_markdown_and_size_overflow() {
    let input = stage_input(Some(1));
    for content in ["[]", "```json\n{\"attribute_changes\": []}\n```"] {
        let error = parse_character_profile_sync_stage_output(
            CharacterProfileSyncStage::AttributeChanges,
            content,
            &input,
            &mut super::contracts::CharacterProfileSyncStageOutputs::default(),
        )
        .expect_err("non-object stage output must fail");
        assert!(error.contains("必须是 JSON 对象"));
    }

    let mut too_long = canonical_attribute_change();
    too_long["change_description"] = json!("长".repeat(121));
    assert_stage_error_contains(
        CharacterProfileSyncStage::AttributeChanges,
        json!({"attribute_changes": [too_long]}),
        &input,
        "最多 120 字",
    );

    let mut too_many_tags = canonical_behavior();
    too_many_tags["behavior_tags"] = json!(["一", "二", "三", "四", "五", "六"]);
    assert_stage_error_contains(
        CharacterProfileSyncStage::BehaviorRecords,
        json!({"behaviors": [too_many_tags]}),
        &input,
        "最多 5 项",
    );
}

#[test]
fn stage_contract_does_not_impose_arbitrary_per_chapter_item_caps() {
    let input = stage_input(Some(1));
    let changes = (0..7)
        .map(|index| {
            let mut change = canonical_attribute_change();
            change["key"] = json!(format!("属性{index}"));
            change
        })
        .collect::<Vec<_>>();
    let behaviors = (0..7)
        .map(|index| {
            let mut behavior = canonical_behavior();
            behavior["behavior_description"] = json!(format!("关键行为{index}"));
            behavior
        })
        .collect::<Vec<_>>();
    let deletion_candidates = (0..4)
        .map(|index| {
            let mut candidate = canonical_deletion_candidate();
            candidate["key"] = json!(format!("旧属性{index}"));
            candidate
        })
        .collect::<Vec<_>>();

    assert_eq!(
        parse_stage_value(
            CharacterProfileSyncStage::AttributeChanges,
            json!({"attribute_changes": changes}),
            &input,
        )
        .expect("valid attribute changes should not be count-limited")["attribute_changes"]
            .as_array()
            .expect("attribute_changes should be an array")
            .len(),
        7
    );
    assert_eq!(
        parse_stage_value(
            CharacterProfileSyncStage::BehaviorRecords,
            json!({"behaviors": behaviors}),
            &input,
        )
        .expect("valid behaviors should not be count-limited")["behaviors"]
            .as_array()
            .expect("behaviors should be an array")
            .len(),
        7
    );
    assert_eq!(
        parse_stage_value(
            CharacterProfileSyncStage::DeletionCandidates,
            json!({"deletion_candidates": deletion_candidates}),
            &input,
        )
        .expect("valid deletion candidates should not be count-limited")["deletion_candidates"]
            .as_array()
            .expect("deletion_candidates should be an array")
            .len(),
        4
    );
}

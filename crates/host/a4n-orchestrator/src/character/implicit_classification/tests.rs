use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use a4n_db::{
    DatabaseConnection,
    workflow::{UpsertWorkflowRunInput, upsert_workflow_run},
};
use a4n_db_models::models::generation::workflow_checkpoint;
use a4n_domain::workflow::WORKFLOW_STATUS_RUNNING;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatMessage, ChatResponse};
use a4n_prompt::rendering::PromptRenderer;
use a4n_test_support::llm::CountingLlmProvider;
use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};

use super::{
    application::apply_implicit_classification_to_behavior,
    contracts::CharacterImplicitClassificationInput, output::parse_implicit_classification_output,
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
        assert_eq!(template_name, super::IMPLICIT_CLASSIFICATION_KIND);
        Ok("system prompt".to_string())
    }
}

async fn setup_context(
    llm: Option<Arc<CountingLlmProvider>>,
) -> (Arc<DatabaseConnection>, OrchestratorContext) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let builder = TestOrchestratorBuilder::new(Arc::clone(&database));
    let context = match llm {
        Some(llm) => builder.with_llm(llm).build().await,
        None => builder.build().await,
    };
    (database, context)
}

async fn create_workflow(database: &DatabaseConnection) -> i64 {
    let sequence = NEXT_WORKFLOW_KEY.fetch_add(1, Ordering::SeqCst);
    upsert_workflow_run(
        database,
        UpsertWorkflowRunInput {
            business_key: format!(
                "implicit-classification-test-{}-{sequence}",
                std::process::id()
            ),
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

fn behavior() -> Value {
    json!({
        "chapter_number": 3,
        "behavior_type": "action",
        "behavior_description": "保护陌生少女",
        "original_text": "林逸挡在少女身前",
        "behavior_tags": ["保护", "冲动"],
        "classification_results": {},
        "related_characters": ["少女"],
        "impact_analysis": ""
    })
}

fn baseline_state() -> Value {
    json!({
        "explicit_attributes": {},
        "implicit_attributes": {"谨慎": "遇事先观察"},
        "social_attributes": {}
    })
}

fn canonical_response() -> String {
    json!({
        "classifications": {"谨慎": "non-conform"},
        "reasoning": "保护他人但没有先观察环境。",
        "suggested_new_attributes": [
            {"key": "见义勇为", "value": "遇到弱者受威胁时主动保护"}
        ]
    })
    .to_string()
}

fn counting_provider(response: String, finish_reason: &str) -> Arc<CountingLlmProvider> {
    Arc::new(CountingLlmProvider::new(ChatResponse {
        message: ChatMessage::assistant(response),
        usage: None,
        finish_reason: Some(finish_reason.to_string()),
        reasoning_content: None,
    }))
}

#[test]
fn output_accepts_canonical_object() {
    let parsed = parse_implicit_classification_output(
        r#"{
              "classifications": {
                "谨慎": "non-conform",
                "重情义": "conform"
              },
              "reasoning": "行为保护他人但没有先观察环境。",
              "suggested_new_attributes": [
                {
                  "key": "见义勇为",
                  "value": "遇到弱者受威胁时主动保护"
                }
              ]
            }"#,
        &json!({
            "谨慎": "遇事先观察",
            "重情义": "愿意保护他人"
        }),
    )
    .expect("canonical classification output should parse");

    assert_eq!(parsed["classifications"]["谨慎"], json!("non-conform"));
    assert_eq!(parsed["classifications"]["重情义"], json!("conform"));
    assert_eq!(
        parsed["suggested_new_attributes"][0]["key"],
        json!("见义勇为")
    );
}

#[test]
fn output_rejects_legacy_evidence() {
    let error = parse_implicit_classification_output(
        r#"{
              "classifications": {},
              "reasoning": "行为体现新属性。",
              "suggested_new_attributes": [
                {
                  "key": "见义勇为",
                  "value": "遇到弱者受威胁时主动保护",
                  "evidence": "林逸挡在少女身前"
                }
              ]
            }"#,
        &json!({"谨慎": "遇事先观察"}),
    )
    .expect_err("legacy evidence should fail");

    assert!(error.contains("evidence 已废弃"));
}

#[test]
fn output_rejects_unknown_top_level_field() {
    let error = parse_implicit_classification_output(
        r#"{
              "classifications": {},
              "reasoning": "无相关属性。",
              "suggested_new_attributes": [],
              "notes": "多余说明"
            }"#,
        &json!({"谨慎": "遇事先观察"}),
    )
    .expect_err("unknown top-level field should fail");

    assert!(error.contains("未支持字段 `notes`"));
}

#[test]
fn output_rejects_unknown_suggestion_field() {
    let error = parse_implicit_classification_output(
        r#"{
              "classifications": {},
              "reasoning": "行为体现新属性。",
              "suggested_new_attributes": [
                {
                  "key": "果断",
                  "value": "遇事快速决定",
                  "confidence": 0.8
                }
              ]
            }"#,
        &json!({"谨慎": "遇事先观察"}),
    )
    .expect_err("unknown suggested attribute field should fail");

    assert!(error.contains("未支持字段 `confidence`"));
}

#[test]
fn output_rejects_unknown_attribute() {
    let error = parse_implicit_classification_output(
        r#"{
              "classifications": {
                "谨慎": "conform",
                "孤僻": "non-conform"
              },
              "reasoning": "包含不存在的属性。",
              "suggested_new_attributes": []
            }"#,
        &json!({"谨慎": "遇事先观察"}),
    )
    .expect_err("classification for unknown attribute must fail");

    assert!(error.contains("不属于当前隐性属性"));
}

#[test]
fn output_rejects_invalid_classification_value() {
    let error = parse_implicit_classification_output(
        r#"{
              "classifications": {
                "谨慎": "unknown"
              },
              "reasoning": "分类值不合法。",
              "suggested_new_attributes": []
            }"#,
        &json!({"谨慎": "遇事先观察"}),
    )
    .expect_err("classification values must be strict");

    assert!(error.contains("conform/non-conform"));
}

#[test]
fn application_adds_each_new_attribute_once() {
    let mut behavior = json!({
        "chapter_number": 3,
        "behavior_type": "action",
        "behavior_description": "保护陌生少女",
        "original_text": "林逸挡在少女身前",
        "behavior_tags": ["保护", "冲动"],
        "classification_results": {},
        "related_characters": ["少女"],
        "impact_analysis": ""
    });
    let parsed = json!({
        "classifications": {"谨慎": "non-conform"},
        "reasoning": "保护他人但缺少观察。",
        "suggested_new_attributes": [
            {"key": "见义勇为", "value": "遇到弱者受威胁时主动保护"},
            {"key": "见义勇为", "value": "重复建议应被忽略"},
            {"key": "谨慎", "value": "已有属性应被忽略"}
        ]
    });
    let mut current_state = json!({
        "explicit_attributes": {},
        "implicit_attributes": {"谨慎": "遇事先观察"},
        "social_attributes": {}
    });
    let suggestions =
        apply_implicit_classification_to_behavior(&mut behavior, &parsed, &mut current_state, 7)
            .expect("classification should apply");

    assert_eq!(
        behavior["classification_results"]["谨慎"],
        json!("non-conform")
    );
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0]["key"], json!("见义勇为"));
    assert_eq!(suggestions[0]["sequence"], json!(7));
    assert_eq!(suggestions[0]["evidence"], json!("林逸挡在少女身前"));
    assert_eq!(
        current_state["implicit_attributes"]["见义勇为"],
        json!("遇到弱者受威胁时主动保护")
    );
    assert_eq!(
        current_state["implicit_attributes"]["谨慎"],
        json!("遇事先观察")
    );
}

#[tokio::test]
async fn application_skips_llm_when_no_behavior_or_implicit_attribute_exists() {
    let (_database, context) = setup_context(None).await;
    let no_behavior = context
        .classify_character_sync_behaviors_validated(
            CharacterImplicitClassificationInput {
                project_id: 1,
                workflow_run_id: 1,
                profile_id: 1,
                profile_name: "林夏",
                changes: &[json!({
                    "chapter_number": 2,
                    "category": "explicit",
                    "key": "位置",
                    "operation": "modify",
                    "new_value": "落霞镇",
                    "change_description": "抵达落霞镇",
                    "event_cause": "赶路抵达",
                    "assertion": "observed",
                    "evidence": "他终于看见落霞镇的牌坊"
                })],
                behaviors: Vec::new(),
                final_attributes: json!({
                    "explicit_attributes": {"位置": "山门"},
                    "implicit_attributes": {},
                    "social_attributes": {}
                }),
            },
            &TestPromptRenderer,
        )
        .await
        .expect("empty behavior application should only project changes");
    assert_eq!(
        no_behavior.final_attributes["explicit_attributes"]["位置"],
        json!("落霞镇")
    );
    assert!(no_behavior.classification_suggestions.is_empty());

    let no_implicit_attribute = context
        .classify_character_sync_behaviors_validated(
            CharacterImplicitClassificationInput {
                project_id: 1,
                workflow_run_id: 1,
                profile_id: 1,
                profile_name: "林夏",
                changes: &[],
                behaviors: vec![behavior()],
                final_attributes: json!({
                    "explicit_attributes": {},
                    "implicit_attributes": {},
                    "social_attributes": {}
                }),
            },
            &TestPromptRenderer,
        )
        .await
        .expect("empty implicit attributes should not call LLM");
    assert_eq!(
        no_implicit_attribute.behaviors[0]["classification_results"],
        json!({})
    );
    assert!(no_implicit_attribute.classification_suggestions.is_empty());
}

#[tokio::test]
async fn application_persists_restores_and_revalidates_checkpoint() {
    let provider = counting_provider(canonical_response(), "stop");
    let (database, context) = setup_context(Some(Arc::clone(&provider))).await;
    let workflow_run_id = create_workflow(&database).await;
    let input = CharacterImplicitClassificationInput {
        project_id: 1,
        workflow_run_id,
        profile_id: 1,
        profile_name: "林夏",
        changes: &[],
        behaviors: vec![behavior()],
        final_attributes: baseline_state(),
    };

    let output = context
        .classify_character_sync_behaviors_validated(input.clone(), &TestPromptRenderer)
        .await
        .expect("classification application should succeed");
    assert_eq!(provider.stream_call_count(), 1);
    assert_eq!(
        output.behaviors[0]["classification_results"]["谨慎"],
        json!("non-conform")
    );
    assert_eq!(output.classification_suggestions.len(), 1);
    assert_eq!(
        output.final_attributes["implicit_attributes"]["见义勇为"],
        json!("遇到弱者受威胁时主动保护")
    );

    let checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .one(database.as_ref())
        .await
        .expect("checkpoint query should succeed")
        .expect("classification checkpoint should exist");
    let checkpoint_json = checkpoint
        .checkpoint_json
        .as_ref()
        .expect("classification checkpoint payload should exist");
    assert_eq!(checkpoint.step_key, "implicit_classification:1:3:0");
    assert_eq!(
        checkpoint_json["parsed_output"]["classifications"]["谨慎"],
        json!("non-conform")
    );

    let restored = context
        .classify_character_sync_behaviors_validated(input.clone(), &TestPromptRenderer)
        .await
        .expect("matching checkpoint should restore");
    assert_eq!(restored, output);
    assert_eq!(provider.stream_call_count(), 1);

    let response_content = checkpoint_json["response_content"].clone();
    let mut active: workflow_checkpoint::ActiveModel = checkpoint.into();
    active.checkpoint_json = Set(Some(json!({
        "parsed_output": {
            "classifications": {"谨慎": "conform"},
            "reasoning": "被篡改的断点",
            "suggested_new_attributes": []
        },
        "response_content": response_content,
        "usage": null,
        "finish_reason": "stop"
    })));
    active
        .update(database.as_ref())
        .await
        .expect("checkpoint tamper setup should succeed");

    let error = context
        .classify_character_sync_behaviors_validated(input, &TestPromptRenderer)
        .await
        .expect_err("mismatched parsed_output must be rejected");
    assert!(
        error
            .to_string()
            .contains("checkpoint_parsed_output_mismatch")
    );
    assert_eq!(provider.stream_call_count(), 1);
}

#[tokio::test]
async fn application_rejects_truncated_llm_response_without_checkpoint() {
    let provider = counting_provider(canonical_response(), "length");
    let (database, context) = setup_context(Some(Arc::clone(&provider))).await;
    let workflow_run_id = create_workflow(&database).await;

    let error = context
        .classify_character_sync_behaviors_validated(
            CharacterImplicitClassificationInput {
                project_id: 1,
                workflow_run_id,
                profile_id: 1,
                profile_name: "林夏",
                changes: &[],
                behaviors: vec![behavior()],
                final_attributes: baseline_state(),
            },
            &TestPromptRenderer,
        )
        .await
        .expect_err("truncated classification response must fail");
    assert!(matches!(&error, AppError::InvalidArgument(_)));
    assert!(error.to_string().contains("模型输出被截断"));
    assert_eq!(provider.stream_call_count(), 1);

    let checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .one(database.as_ref())
        .await
        .expect("checkpoint query should succeed");
    assert!(checkpoint.is_none());
}

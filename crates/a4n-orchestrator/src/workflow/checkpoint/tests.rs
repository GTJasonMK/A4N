use super::super::WORKFLOW_STATUS_COMPLETED;
use super::contracts::{
    CompletedWorkflowCheckpointContractError, CompletedWorkflowCheckpointInput,
    required_checkpoint_field, required_checkpoint_json,
};
use super::persistence::completed_workflow_checkpoint_upsert_input;
use a4n_core::models::generation::workflow_checkpoint;
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use chrono::Utc;
use serde_json::{Value, json};

fn checkpoint(checkpoint_json: Option<Value>) -> workflow_checkpoint::Model {
    let now = Utc::now().naive_utc();
    workflow_checkpoint::Model {
        id: 1,
        workflow_run_id: 7,
        step_key: "chapter.sync".to_string(),
        step_order: 3,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        input_fingerprint_sha256: None,
        input_fingerprint_json: None,
        checkpoint_json,
        artifacts_json: None,
        generation_run_ids_json: None,
        error_message: None,
        created_at: now,
        updated_at: now,
        completed_at: Some(now),
    }
}

#[test]
fn completed_checkpoint_contract_preserves_step_and_field() {
    let missing_json = required_checkpoint_json(&checkpoint(None), "chapter.sync")
        .expect_err("missing checkpoint JSON must fail");
    assert!(matches!(
        missing_json,
        CompletedWorkflowCheckpointContractError::MissingCheckpointJson { step_key }
            if step_key == "chapter.sync"
    ));

    let missing_field = required_checkpoint_field(&json!({}), "chapter.sync", "profile_ids")
        .expect_err("missing checkpoint field must fail");
    assert!(matches!(
        missing_field,
        CompletedWorkflowCheckpointContractError::MissingCheckpointField { step_key, field }
            if step_key == "chapter.sync" && field == "profile_ids"
    ));
}

#[test]
fn optional_workflow_run_controls_completed_checkpoint_persistence() {
    let without_run =
        completed_workflow_checkpoint_upsert_input(CompletedWorkflowCheckpointInput {
            workflow_run_id: None,
            step_key: "chapter.sync".to_string(),
            step_order: 3,
            input_fingerprint_json: json!({"chapter_id": 9}),
            checkpoint_json: Some(json!({"profile_ids": [1, 2]})),
        })
        .expect("optional workflow checkpoint input");
    assert!(without_run.is_none());

    let with_run = completed_workflow_checkpoint_upsert_input(CompletedWorkflowCheckpointInput {
        workflow_run_id: Some(7),
        step_key: "chapter.sync".to_string(),
        step_order: 3,
        input_fingerprint_json: json!({"chapter_id": 9}),
        checkpoint_json: Some(json!({"profile_ids": [1, 2]})),
    })
    .expect("completed workflow checkpoint input")
    .expect("workflow run should produce persistence input");

    assert_eq!(with_run.workflow_run_id, 7);
    assert_eq!(with_run.step_key, "chapter.sync");
    assert_eq!(with_run.step_order, 3);
    assert_eq!(with_run.status, WORKFLOW_STATUS_COMPLETED);
    assert!(with_run.completed);
    assert!(with_run.input_fingerprint_sha256.is_some());
    assert_eq!(
        with_run.checkpoint_json,
        Some(json!({"profile_ids": [1, 2]}))
    );

    let completion_marker =
        completed_workflow_checkpoint_upsert_input(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(7),
            step_key: "chapter.marker".to_string(),
            step_order: 4,
            input_fingerprint_json: json!({"chapter_id": 9}),
            checkpoint_json: None,
        })
        .expect("completed workflow marker input")
        .expect("workflow run should persist completion marker");
    assert!(completion_marker.checkpoint_json.is_none());
}

#[test]
fn checkpoint_trace_fields_are_attached_together() {
    let mut request = ChatRequest::system_user("system", "user", None, None, None)
        .with_trace_context(LlmTraceContext {
            project_id: Some(1),
            root_project_id: None,
            workflow: "test".to_string(),
            step_key: "step".to_string(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: None,
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });

    super::attach_llm_call_checkpoint_trace(&mut request, 7, "chapter.sync", &json!({"id": 1}))
        .expect("trace contract should be valid");

    let trace = request.trace_context.expect("trace context");
    assert_eq!(trace.workflow_run_id, Some(7));
    assert_eq!(trace.call_key.as_deref(), Some("chapter.sync"));
    assert_eq!(
        trace.input_fingerprint_sha256.as_deref().map(str::len),
        Some(64)
    );
}

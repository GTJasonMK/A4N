use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_llm_provider_contract::provider::UsageInfo;
use serde_json::Value;

use super::*;

fn response(content: &str) -> ChatResponse {
    ChatResponse {
        message: ChatMessage::assistant(content),
        finish_reason: Some("stop".into()),
        reasoning_content: None,
        usage: Some(UsageInfo {
            prompt_tokens: 10,
            completion_tokens: 5,
            total_tokens: 15,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
            prompt_cache_write_tokens: None,
        }),
    }
}

fn parse(raw: &str) -> Result<i64, AppError> {
    let value: Value = super::super::parse_strict_json_response(raw, "test")?;
    value["id"]
        .as_i64()
        .filter(|id| *id > 0)
        .ok_or_else(|| AppError::LlmError("id 必须是正整数".into()))
}

fn request() -> ChatRequest {
    ChatRequest::system_user("遵循整数合同", "原始任务和来源", None, Some(100))
        .with_json_response_format()
}

#[tokio::test]
async fn correction_has_its_own_fingerprint_and_replans_the_added_feedback() {
    let request =
        request().with_trace_context(a4n_llm_provider_contract::provider::LlmTraceContext {
            project_id: Some(1),
            root_project_id: Some(1),
            workflow: "test".into(),
            step_key: "stage".into(),
            generation_run_id: None,
            workflow_run_id: Some(1),
            call_key: Some("call".into()),
            input_fingerprint_sha256: Some("0".repeat(64)),
            context_plan_json: Some(json!({"old_plan":true})),
            context_pack_refs_json: Some(json!({"old_packs":true})),
        });
    correct_llm_response(
        &request,
        response("{}"),
        "test",
        None,
        parse,
        |corrected| async move {
            let trace = corrected.trace_context.unwrap();
            assert_eq!(trace.call_key.as_deref(), Some("call.correction.1"));
            let fingerprint = trace.input_fingerprint_sha256.unwrap();
            assert_eq!(fingerprint.len(), 64);
            assert_ne!(fingerprint, "0".repeat(64));
            assert!(trace.context_plan_json.is_none());
            assert!(trace.context_pack_refs_json.is_none());
            Ok(response(r#"{"id":1}"#))
        },
    )
    .await
    .unwrap();
    assert!(request.trace_context.unwrap().context_plan_json.is_some());
}

#[tokio::test]
async fn valid_output_does_not_call_model_again() {
    let (_, id) = correct_llm_response(
        &request(),
        response(r#"{"id":1}"#),
        "test",
        None,
        parse,
        |_| async { panic!("valid output must not incur another model call") },
    )
    .await
    .unwrap();
    assert_eq!(id, 1);
}

#[tokio::test(start_paused = true)]
async fn correction_can_finish_after_ninety_seconds_without_a_second_deadline() {
    let (_, id) =
        correct_llm_response(&request(), response("{}"), "test", None, parse, |_| async {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            Ok(response(r#"{"id":1}"#))
        })
        .await
        .unwrap();
    assert_eq!(id, 1);
}

#[tokio::test]
async fn long_json_with_trailing_content_is_rejected_and_its_tail_reaches_correction() {
    let trailing = r#"{"unexpected":"TRAILING_JSON_MARKER"}"#;
    let invalid = format!(
        r#"{{"id":1,"padding":"{}"}}{trailing}"#,
        "原文".repeat(12_000)
    );
    assert!(
        parse(&invalid)
            .unwrap_err()
            .to_string()
            .contains("trailing characters")
    );
    let (_, id) = correct_llm_response(
        &request(),
        response(&invalid),
        "test",
        None,
        parse,
        |corrected| async move {
            let feedback: Value =
                serde_json::from_str(&corrected.messages.last().unwrap().content).unwrap();
            let excerpt = feedback["previous_response_excerpt"].as_str().unwrap();
            assert!(excerpt.starts_with(r#"{"id":1,"padding":""#));
            assert!(excerpt.ends_with(trailing));
            assert_eq!(excerpt.chars().count(), MAX_RESPONSE_EXCERPT_CHARS);
            assert_eq!(feedback["previous_response_chars"], invalid.chars().count());
            assert!(
                feedback["validation_error"]
                    .as_str()
                    .unwrap()
                    .contains("trailing characters")
            );
            Ok(response(r#"{"id":2}"#))
        },
    )
    .await
    .unwrap();
    assert_eq!(id, 2, "only the validated replacement may be adopted");
}

#[test]
fn response_excerpts_keep_short_outputs_and_bound_multibyte_head_and_tail() {
    for count in [
        0,
        MAX_RESPONSE_EXCERPT_CHARS - 1,
        MAX_RESPONSE_EXCERPT_CHARS,
        MAX_RESPONSE_EXCERPT_CHARS + 1,
    ] {
        let content = "中".repeat(count);
        let excerpt = response_excerpt(&content);
        assert!(excerpt.chars().count() <= MAX_RESPONSE_EXCERPT_CHARS);
        if count <= MAX_RESPONSE_EXCERPT_CHARS {
            assert_eq!(excerpt, content);
        } else {
            assert!(excerpt.starts_with('中') && excerpt.ends_with('中'));
            assert!(excerpt.contains("中间内容省略"));
        }
    }
}

#[tokio::test]
async fn correction_repeats_schema_for_missing_state_without_filling_it() {
    let schema = json!({
        "type":"object", "required":["occurrences"],
        "properties":{"occurrences":{"type":"array","items":{
            "type":"object", "required":["entry_state","exit_state"],
            "properties":{"entry_state":{"type":"object"},"exit_state":{"type":"object"}}
        }}}
    });
    let request = request()
        .with_json_schema_response_format("scene_states", schema.clone(), false)
        .unwrap();
    let initial = json!({"occurrences":[{"entry_state":{"door":"open"}}]}).to_string();
    let expected = json!({"occurrences":[{
        "entry_state":{"door":"open"}, "exit_state":{"door":{"locked":true}}
    }]});
    let (_, parsed) = correct_llm_response(
        &request,
        response(&initial),
        "scene_occurrence_extraction",
        None,
        |raw| {
            let value: Value = super::super::parse_strict_json_response(raw, "states")?;
            a4n_domain::json_contract::required_object_value(
                &value["occurrences"][0],
                "exit_state",
                "occurrences[0]",
            )
            .map_err(AppError::LlmError)?;
            Ok(value)
        },
        |corrected| {
            let feedback: Value =
                serde_json::from_str(&corrected.messages.last().unwrap().content).unwrap();
            assert_eq!(feedback["output_schema"], schema);
            assert_eq!(feedback["previous_response_excerpt"], initial);
            assert!(
                feedback["validation_error"]
                    .as_str()
                    .unwrap()
                    .contains("exit_state")
            );
            assert_eq!(corrected.response_format, request.response_format);
            assert_eq!(corrected.max_tokens, request.max_tokens);
            std::future::ready(Ok(response(&expected.to_string())))
        },
    )
    .await
    .unwrap();
    assert_eq!(parsed, expected);
}

#[tokio::test]
async fn invalid_output_gets_one_feedback_call_and_usage_is_accumulated() {
    let request = request().with_context_manifest_json(json!({"test": "original source"}));
    let expected = &request;
    let (output, id) = correct_llm_response(
        &request,
        response(r#"{"id":"1"}"#),
        "test",
        None,
        parse,
        |corrected| async move {
            assert_eq!(corrected.messages.len(), 3);
            assert_eq!(corrected.messages[0].content, expected.messages[0].content);
            assert_eq!(corrected.messages[1].content, expected.messages[1].content);
            assert_eq!(
                corrected.context_manifest_json,
                expected.context_manifest_json
            );
            assert_eq!(corrected.max_tokens, expected.max_tokens);
            let feedback: Value = serde_json::from_str(&corrected.messages[2].content).unwrap();
            assert_eq!(feedback["kind"], "model_output_correction");
            assert_eq!(feedback["max_correction_attempts"], 1);
            assert!(
                feedback["validation_error"]
                    .as_str()
                    .unwrap()
                    .contains("正整数")
            );
            Ok(response(r#"{"id":1}"#))
        },
    )
    .await
    .unwrap();
    assert_eq!(id, 1);
    assert_eq!(output.usage.unwrap().total_tokens, 30);
}

#[tokio::test]
async fn repeated_invalid_output_stops_and_preserves_both_errors() {
    let calls = AtomicUsize::new(0);
    let error = correct_llm_response(
        &request(),
        response(r#"{"id":"1"}"#),
        "test",
        None,
        parse,
        |_| async {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(response("not json"))
        },
    )
    .await
    .unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let error = error.to_string();
    assert!(error.contains("1 次输出纠正"));
    assert!(error.contains("id 必须是正整数"));
    assert!(error.contains("严格 JSON"));
}

#[tokio::test]
async fn truncated_output_is_not_blindly_retried() {
    let mut truncated = response("partial");
    truncated.finish_reason = Some("length".into());
    assert!(
        correct_llm_response(&request(), truncated, "test", None, parse, |_| async {
            panic!("truncation needs a budget/input change")
        })
        .await
        .is_err()
    );
}

#[tokio::test]
async fn state_failure_does_not_spend_another_call() {
    let error = correct_llm_response(
        &request(),
        response("{}"),
        "test",
        None,
        |_| {
            Err::<(), _>(AppError::InvalidState {
                current: "stale_source".into(),
                operation: "test".into(),
            })
        },
        |_| async { panic!("state errors are not model output errors") },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AppError::InvalidState { .. }));
}

#[tokio::test]
async fn cancellation_stops_pending_correction() {
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let error = correct_llm_response(
        &request(),
        response("{}"),
        "test",
        Some(&cancel),
        parse,
        |_| async move {
            token.cancel();
            std::future::pending::<Result<ChatResponse, AppError>>().await
        },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AppError::Cancelled));
}

#[tokio::test]
async fn cancelled_call_does_not_validate_or_restart() {
    let cancel = CancellationToken::new();
    cancel.cancel();
    let error = correct_llm_response(
        &request(),
        response("{}"),
        "test",
        Some(&cancel),
        |_| -> Result<(), AppError> { panic!("cancelled work must stop") },
        |_| async { panic!("cancelled work must not call provider") },
    )
    .await
    .unwrap_err();
    assert!(matches!(error, AppError::Cancelled));
}

#[tokio::test]
async fn tool_turns_are_left_to_the_agent_budget_and_recovery_policy() {
    let mut request = request();
    request.tools = Some(vec![json!({"type":"function"})]);
    let error = correct_llm_response(&request, response("{}"), "test", None, parse, |_| async {
        panic!("tool turns must not incur unaccounted model rounds")
    })
    .await
    .unwrap_err();
    assert!(error.to_string().contains("正整数"));
}

#[tokio::test]
async fn corrective_transport_failure_is_visible_and_not_retried_again() {
    let calls = Arc::new(AtomicUsize::new(0));
    let error = correct_llm_response(&request(), response("{}"), "test", None, parse, |_| async {
        calls.fetch_add(1, Ordering::SeqCst);
        Err(AppError::LlmError("HTTP 429".into()))
    })
    .await
    .unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(error.to_string().contains("HTTP 429"));
}

#[test]
fn unknown_usage_is_not_reported_as_a_complete_total() {
    assert!(merge_usage(response("{}").usage, None).unwrap().is_none());
    let mut huge = response("{}").usage.unwrap();
    huge.total_tokens = usize::MAX;
    assert!(merge_usage(Some(huge), response("{}").usage).is_err());
}

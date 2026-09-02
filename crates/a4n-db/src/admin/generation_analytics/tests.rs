use super::aggregate::build_generation_call_analytics;
use super::query::validate_generation_call_analytics_input;
use super::*;
use sea_orm::EntityTrait;
use serde_json::json;

#[test]
fn analytics_input_rejects_unknown_fields_at_the_wire_boundary() {
    let error = serde_json::from_value::<GenerationCallAnalyticsInput>(json!({
        "days": 30,
        "recent_limit": 20,
        "unexpected": true
    }))
    .expect_err("unknown analytics input fields must fail");

    assert!(error.to_string().contains("unknown field `unexpected`"));
}

#[test]
fn analytics_input_rejects_out_of_range_days_as_invalid_argument() {
    let error = validate_generation_call_analytics_input(GenerationCallAnalyticsInput {
        days: GENERATION_ANALYTICS_MAX_DAYS + 1,
        recent_limit: 1,
    })
    .expect_err("days above the supported range must fail");

    assert!(matches!(&error, AdminDbError::InvalidArgument(_)));
    assert_eq!(error.to_string(), "days 必须在 1..=180 之间");
}

#[test]
fn analytics_input_rejects_out_of_range_recent_limit_as_invalid_argument() {
    let error = validate_generation_call_analytics_input(GenerationCallAnalyticsInput {
        days: 1,
        recent_limit: 0,
    })
    .expect_err("zero recent limit must fail");

    assert!(matches!(&error, AdminDbError::InvalidArgument(_)));
    assert_eq!(error.to_string(), "recent_limit 必须在 1..=200 之间");
}

#[tokio::test]
async fn generation_analytics_query_preserves_output_without_loading_large_json_payloads() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let now = Utc::now().naive_utc();
    let run = generation_run::Model {
        id: 11,
        project_id: 1,
        root_project_id: 1,
        workflow: "video_generation".to_string(),
        step_key: "render".to_string(),
        title: "测试视频".to_string(),
        model_kind: "video".to_string(),
        provider: Some("mock".to_string()),
        model: Some("mock-video".to_string()),
        status: "completed".to_string(),
        workflow_run_id: None,
        call_key: None,
        input_fingerprint_sha256: None,
        input_fingerprint_json: Some(json!({"large": [1, 2, 3]})),
        raw_response_json: Some(json!({"large": [1, 2, 3]})),
        parsed_output_json: Some(json!({"large": [1, 2, 3]})),
        validation_json: Some(json!({"ok": true})),
        artifact_refs_json: Some(json!({"asset": 1})),
        attempt: 1,
        prompt_template_names_json: Some(json!(["video"])),
        prompt_template_hashes_json: Some(json!(["hash"])),
        rendered_messages_json: Some(json!([{"role": "user"}])),
        rendered_prompts_json: Some(json!(["prompt"])),
        input_variables_json: Some(json!({"text": "input"})),
        source_refs_json: Some(json!([1])),
        media_refs_json: Some(json!([2])),
        request_json: Some(json!({"request": true})),
        response_summary_json: Some(json!({"summary": true})),
        error_message: None,
        created_at: now,
        completed_at: Some(now),
    };
    let run: generation_run::ActiveModel = run.into();
    generation_run::Entity::insert(run)
        .exec(&db)
        .await
        .expect("generation run fixture should insert");

    let mut call = llm_call(12, "video_prompt", 100, 80, 20);
    call.generation_run_id = Some(11);
    call.context_pack_refs_json = Some(json!({"large": [1, 2, 3]}));
    let call: llm_call_trace::ActiveModel = call.into();
    llm_call_trace::Entity::insert(call)
        .exec(&db)
        .await
        .expect("llm call fixture should insert");

    let result = super::query::generation_call_analytics(
        &db,
        GenerationCallAnalyticsInput {
            days: 1,
            recent_limit: 10,
        },
    )
    .await
    .expect("generation analytics query should succeed");
    assert_eq!(result.recent_llm_calls.len(), 1);
    assert_eq!(result.recent_llm_calls[0].id, 12);
    assert_eq!(result.recent_media_calls.len(), 1);
    assert_eq!(result.recent_media_calls[0].id, 11);
    assert_eq!(result.recent_media_calls[0].title, "测试视频");
}

fn llm_call(id: i64, workflow: &str, total: i32, hit: i32, miss: i32) -> llm_call_trace::Model {
    llm_call_trace::Model {
        id,
        generation_run_id: None,
        workflow_run_id: None,
        call_key: None,
        input_fingerprint_sha256: None,
        project_id: None,
        root_project_id: None,
        workflow: workflow.to_string(),
        step_key: "step".to_string(),
        provider: Some("deepseek".to_string()),
        model: Some("deepseek-chat".to_string()),
        context_plan_json: Some(json!({})),
        context_pack_refs_json: None,
        prompt_tokens: Some(total - 10),
        completion_tokens: Some(10),
        total_tokens: Some(total),
        prompt_cache_hit_tokens: Some(hit),
        prompt_cache_miss_tokens: Some(miss),
        prompt_cache_hit_ratio: None,
        duration_ms: Some(1000),
        streamed: false,
        status: "completed".to_string(),
        error_kind: None,
        error_message: None,
        finish_reason: Some("stop".to_string()),
        created_at: Utc::now().naive_utc(),
    }
}

fn llm_call_with_context_plan(
    id: i64,
    workflow: &str,
    status: &str,
    prompt_tokens: i32,
    plan: serde_json::Value,
) -> llm_call_trace::Model {
    let mut call = llm_call(id, workflow, prompt_tokens + 10, 0, 0);
    call.status = status.to_string();
    call.prompt_tokens = Some(prompt_tokens);
    call.context_plan_json = Some(plan);
    call
}

#[test]
fn analytics_aggregates_llm_cache_ratio() {
    let now = Utc::now().naive_utc();
    let result = build_generation_call_analytics(
        now,
        now - Duration::days(30),
        30,
        10,
        vec![
            llm_call(1, "video_prompt", 100, 80, 20),
            llm_call(2, "video_prompt", 50, 20, 30),
        ],
        vec![],
    );
    let llm = result
        .overview
        .iter()
        .find(|row| row.kind == "llm")
        .expect("llm overview should exist");
    assert_eq!(llm.calls, 2);
    assert_eq!(llm.total_tokens, 150);
    assert_eq!(llm.prompt_cache_hit_tokens, 100);
    assert_eq!(llm.prompt_cache_miss_tokens, 50);
    assert_eq!(llm.prompt_cache_hit_ratio, Some(100.0 / 150.0));
}

#[test]
fn analytics_aggregates_llm_context_quality_metrics() {
    let now = Utc::now().naive_utc();
    let mut failed = llm_call_with_context_plan(
        2,
        "video_prompt",
        "failed",
        50,
        json!({
            "stable_prefix_tokens": 80,
            "dynamic_delta_tokens": 20,
            "estimated_input_tokens": 100,
            "dropped_optional_pack_count": 1
        }),
    );
    failed.error_kind = Some("context_budget".to_string());
    failed.error_message = Some("当前任务动态输入超过模型可用输入窗口".to_string());
    let result = build_generation_call_analytics(
        now,
        now - Duration::days(30),
        30,
        10,
        vec![
            llm_call_with_context_plan(
                1,
                "video_prompt",
                "completed",
                100,
                json!({
                    "stable_prefix_tokens": 120,
                    "dynamic_delta_tokens": 40,
                    "estimated_input_tokens": 160,
                    "dropped_optional_pack_count": 0
                }),
            ),
            failed,
        ],
        vec![],
    );
    let overview = result
        .overview
        .iter()
        .find(|row| row.kind == "llm")
        .expect("llm overview should exist");
    assert_eq!(overview.calls, 2);
    assert_eq!(overview.success, 1);
    assert_eq!(overview.failed, 1);

    let workflow = result
        .llm_by_workflow
        .iter()
        .find(|row| row.workflow == "video_prompt")
        .expect("workflow row should exist");
    assert_eq!(workflow.calls, 2);
    assert_eq!(workflow.success, 1);
    assert_eq!(workflow.failed, 1);
    assert_eq!(workflow.avg_stable_prefix_tokens, Some(100));
    assert_eq!(workflow.avg_dynamic_delta_tokens, Some(30));
    assert_eq!(workflow.avg_estimated_input_tokens, Some(130));
    assert_eq!(workflow.dropped_optional_pack_count, 1);
    assert_eq!(workflow.context_over_budget_failure_count, 1);
    assert_eq!(
        workflow.estimated_vs_actual_prompt_tokens_ratio,
        Some(((160.0 / 100.0) + (100.0 / 50.0)) / 2.0)
    );

    let recent = result
        .recent_llm_calls
        .iter()
        .find(|row| row.id == 2)
        .expect("failed call should be recent");
    assert_eq!(recent.status, "failed");
    assert_eq!(recent.stable_prefix_tokens, Some(80));
    assert_eq!(recent.dynamic_delta_tokens, Some(20));
    assert_eq!(recent.estimated_input_tokens, Some(100));
    assert_eq!(recent.dropped_optional_pack_count, 1);
}

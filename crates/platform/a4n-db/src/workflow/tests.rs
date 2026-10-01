use super::*;

#[test]
fn workflow_read_error_keeps_database_source() {
    let error =
        WorkflowReadError::Database(sea_orm::DbErr::Custom("workflow query failed".to_string()));

    assert!(matches!(
        &error,
        WorkflowReadError::Database(sea_orm::DbErr::Custom(message))
            if message == "workflow query failed"
    ));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn workflow_read_error_preserves_argument_and_not_found_messages() {
    let invalid = WorkflowReadError::InvalidArgument("workflow_run_id 必须大于 0".to_string());
    let missing = WorkflowReadError::NotFound("工作流不存在: 9".to_string());
    let invalid_data = WorkflowReadError::InvalidData("未知任务状态: stale".to_string());

    assert_eq!(invalid.to_string(), "workflow_run_id 必须大于 0");
    assert_eq!(missing.to_string(), "工作流不存在: 9");
    assert_eq!(
        invalid_data.to_string(),
        "持久化工作流数据无效: 未知任务状态: stale"
    );
}

#[test]
fn workflow_write_error_preserves_argument_and_database_source() {
    let invalid = WorkflowWriteError::InvalidArgument("step_key 不能为空".to_string());
    let missing =
        WorkflowWriteError::from(WorkflowReadError::NotFound("工作流不存在: 7".to_string()));
    let database = WorkflowWriteError::Database(sea_orm::DbErr::Custom(
        "checkpoint delete failed".to_string(),
    ));

    assert_eq!(invalid.to_string(), "step_key 不能为空");
    assert_eq!(missing.to_string(), "工作流不存在: 7");
    assert!(matches!(
        &database,
        WorkflowWriteError::Database(sea_orm::DbErr::Custom(message))
            if message == "checkpoint delete failed"
    ));
    assert!(std::error::Error::source(&database).is_some());

    let conflict = WorkflowWriteError::StatusConflict {
        current: WORKFLOW_STATUS_CANCELLED.to_string(),
        operation: "完成工作流".to_string(),
    };
    assert!(conflict.to_string().contains("cancelled"));
}

#[test]
fn workflow_query_inputs_support_chapter_filter_and_lightweight_detail() {
    let list: ListWorkflowRunsInput = serde_json::from_value(serde_json::json!({
        "project_id": 19,
        "chapter_id": 3,
        "workflow_kind": "video_segment_plan",
        "limit": 1
    }))
    .expect("chapter-filtered workflow query should deserialize");
    assert_eq!(list.chapter_id, Some(3));

    let detail: GetWorkflowRunInput = serde_json::from_value(serde_json::json!({
        "workflow_run_id": 7,
        "include_generation_runs": false
    }))
    .expect("lightweight workflow detail query should deserialize");
    assert!(!detail.include_generation_runs);
}

#[test]
fn workflow_checkpoint_contract_error_keeps_json_source() {
    let source = serde_json::from_str::<Value>("{").expect_err("invalid JSON should fail");
    let error = WorkflowCheckpointContractError::Serialization(source);

    assert!(error.to_string().contains("工作流断点 JSON 序列化失败"));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn validates_completed_llm_call_checkpoint_contract() {
    for reason in [None, Some("stop"), Some("tool_calls")] {
        validate_completed_llm_call_checkpoint_json(
            &serde_json::json!({
                "parsed_output": {"value": 1},
                "response_content": "{\"value\":1}",
                "usage": null,
                "finish_reason": reason,
            }),
            "test.call",
        )
        .expect("valid checkpoint, including optional metadata omitted by earlier producers");
    }
}

#[test]
fn completed_call_checkpoint_rejects_unsuccessful_finish_reason() {
    for reason in [Some(""), Some("content_filter"), Some("unexpected")] {
        let checkpoint = serde_json::json!({
            "parsed_output": {"value": 1},
            "response_content": "{\"value\":1}",
            "usage": null,
            "finish_reason": reason,
        });
        assert!(
            validate_completed_llm_call_checkpoint_json(&checkpoint, "test.call").is_err(),
            "unsuccessful finish_reason {reason:?} must not be a reusable call"
        );
    }
}

#[test]
fn rejects_completed_llm_call_checkpoint_without_parsed_output() {
    let error = validate_completed_llm_call_checkpoint_json(
        &serde_json::json!({
            "response_content": "{\"value\":1}",
            "usage": null,
            "finish_reason": null,
        }),
        "test.call",
    )
    .expect_err("missing parsed_output must fail");

    assert!(matches!(
        error,
        WorkflowCheckpointContractError::Invalid(message)
            if message.contains("parsed_output")
    ));
}

#[test]
fn rejects_completed_llm_call_checkpoint_with_truncated_finish_reason() {
    let error = validate_completed_llm_call_checkpoint_json(
        &serde_json::json!({
            "parsed_output": {"value": 1},
            "response_content": "{\"value\":1}",
            "usage": null,
            "finish_reason": "length",
        }),
        "test.call",
    )
    .expect_err("truncated response checkpoint must fail");

    assert!(matches!(
        error,
        WorkflowCheckpointContractError::Invalid(message)
            if message.contains("截断")
    ));
}

#[test]
fn rejects_completed_llm_call_checkpoint_with_null_parsed_output() {
    let error = validate_completed_llm_call_checkpoint_json(
        &serde_json::json!({
            "parsed_output": null,
            "response_content": "{\"value\":1}",
            "usage": {"completion_tokens": 100},
            "finish_reason": "stop",
        }),
        "test.call",
    )
    .expect_err("null parsed output must not become a resumable checkpoint");

    assert!(matches!(
        error,
        WorkflowCheckpointContractError::Invalid(message)
            if message.contains("parsed_output 不能为 null")
    ));
}

#[test]
fn rejects_completed_llm_call_checkpoint_with_empty_response_content() {
    let error = validate_completed_llm_call_checkpoint_json(
        &serde_json::json!({
            "parsed_output": {"value": 1},
            "response_content": "  ",
            "usage": {"completion_tokens": 100},
            "finish_reason": "stop",
        }),
        "test.call",
    )
    .expect_err("empty raw response must not become a resumable checkpoint");

    assert!(matches!(
        error,
        WorkflowCheckpointContractError::Invalid(message)
            if message.contains("response_content 不能为空")
    ));
}

#[test]
fn rejects_completed_llm_call_checkpoint_with_unknown_field() {
    let error = validate_completed_llm_call_checkpoint_json(
        &serde_json::json!({
            "parsed_output": {"value": 1},
            "response_content": "{\"value\":1}",
            "usage": null,
            "finish_reason": null,
            "output": {"value": 1},
        }),
        "test.call",
    )
    .expect_err("unknown legacy field must fail");

    assert!(matches!(
        error,
        WorkflowCheckpointContractError::Invalid(message)
            if message.contains("未知字段 output")
    ));
}
use a4n_test_support::workflow::WorkflowRunModelBuilder;

#[test]
fn workflow_checkpoint_summary_preserves_persisted_fields() {
    let created_at = test_timestamp();
    let updated_at = created_at + chrono::Duration::minutes(1);
    let completed_at = updated_at + chrono::Duration::minutes(1);
    let summary = workflow_checkpoint_summary_from_model(
        a4n_db_models::models::generation::workflow_checkpoint::Model {
            id: 11,
            workflow_run_id: 12,
            step_key: "chapter.analysis".to_string(),
            step_order: 4,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some("fingerprint".to_string()),
            input_fingerprint_json: Some(json!({"chapter_id": 13})),
            checkpoint_json: Some(json!({"result": "saved"})),
            artifacts_json: Some(json!({"profile_ids": [14]})),
            generation_run_ids_json: Some(json!([15])),
            error_message: None,
            created_at,
            updated_at,
            completed_at: Some(completed_at),
        },
    );

    assert_eq!(summary.id, 11);
    assert_eq!(summary.workflow_run_id, 12);
    assert_eq!(summary.step_key, "chapter.analysis");
    assert_eq!(summary.step_order, 4);
    assert_eq!(summary.status, WORKFLOW_STATUS_COMPLETED);
    assert_eq!(
        summary.input_fingerprint_sha256.as_deref(),
        Some("fingerprint")
    );
    assert_eq!(
        summary.input_fingerprint_json,
        Some(json!({"chapter_id": 13}))
    );
    assert_eq!(summary.checkpoint_json, Some(json!({"result": "saved"})));
    assert_eq!(summary.artifacts_json, Some(json!({"profile_ids": [14]})));
    assert_eq!(summary.generation_run_ids_json, Some(json!([15])));
    assert_eq!(summary.error_message, None);
    assert_eq!(summary.created_at, created_at.and_utc().to_rfc3339());
    assert_eq!(summary.updated_at, updated_at.and_utc().to_rfc3339());
    assert_eq!(
        summary.completed_at,
        Some(completed_at.and_utc().to_rfc3339())
    );
}
use chrono::NaiveDate;
use serde_json::json;

#[test]
fn workflow_run_progress_summary_uses_calculated_percent_when_missing() {
    let summary = workflow_run_progress_summary(&json!({
        "current": 2,
        "total": 4,
        "message": "处理中",
        "updated_at": "2026-01-02 03:04:05"
    }))
    .expect("valid progress JSON should parse")
    .expect("progress fields are present");

    assert_eq!(summary.current, 2);
    assert_eq!(summary.total, 4);
    assert_eq!(summary.message, "处理中");
    assert_eq!(summary.percent, 50.0);
    assert_eq!(summary.updated_at.as_deref(), Some("2026-01-02 03:04:05"));
}

#[test]
fn workflow_run_progress_summary_prefers_explicit_percent() {
    let summary = workflow_run_progress_summary(&json!({
        "current": 2,
        "total": 4,
        "message": "处理中",
        "percent": 42.5,
        "cancellation_active_task_id": "task-7"
    }))
    .expect("valid progress JSON should parse")
    .expect("progress fields are present");

    assert_eq!(summary.percent, 42.5);
}

#[test]
fn workflow_run_progress_summary_rejects_missing_or_out_of_range_fields() {
    let missing_message = workflow_run_progress_summary(&json!({
        "current": 2,
        "total": 4
    }))
    .expect_err("missing message must fail");
    assert!(missing_message.to_string().contains("message"));

    let overflow = workflow_run_progress_summary(&json!({
        "current": u64::from(u32::MAX) + 1,
        "total": u64::from(u32::MAX) + 1,
        "message": "处理中"
    }))
    .expect_err("u32 overflow must fail");
    assert!(overflow.to_string().contains("progress_json"));

    let invalid_range = workflow_run_progress_summary(&json!({
        "current": 5,
        "total": 4,
        "message": "处理中"
    }))
    .expect_err("current greater than total must fail");
    assert!(invalid_range.to_string().contains("不能大于"));
}

#[test]
fn workflow_run_summary_accepts_snapshot_with_or_without_progress() {
    for with_progress in [false, true] {
        let mut value = json!({"snapshot": {"version": 1, "completed_units": [7]}});
        if with_progress {
            value["current"] = json!(1);
            value["total"] = json!(4);
            value["message"] = json!("处理中");
        }
        let run = WorkflowRunModelBuilder::new(
            51,
            "snapshot-workflow",
            "chapter_shooting_script",
            WORKFLOW_STATUS_INTERRUPTED,
            test_timestamp(),
        )
        .with_progress(value)
        .build();
        let summary =
            workflow_run_summary_from_model(run).expect("durable snapshot must not block reads");
        assert_eq!(summary.id, 51);
        assert_eq!(summary.progress.is_some(), with_progress);
        assert_eq!(
            summary.progress_json,
            Some(if with_progress {
                json!({
                    "snapshot": { "version": 1, "completed_units": [7] },
                    "current": 1,
                    "total": 4,
                    "message": "处理中",
                })
            } else {
                json!({ "snapshot": { "version": 1, "completed_units": [7] } })
            })
        );
        if let Some(progress) = summary.progress {
            assert_eq!(progress.current, 1);
            assert_eq!(progress.total, 4);
            assert_eq!(progress.percent, 25.0);
            assert_eq!(progress.message, "处理中");
        }
    }
}

#[test]
fn workflow_run_progress_summary_rejects_malformed_snapshot_and_progress() {
    for value in [
        json!({"snapshot": null}),
        json!({"snapshot": []}),
        json!({"snapshot": "invalid"}),
        json!({"snapshot": {}, "current": 0, "total": 1}),
        json!({"snapshot": {}, "legacy": true}),
        json!({"current": 0, "total": 1, "message": "", "legacy": true}),
        json!({"snapshot": {}, "current": 0, "total": 1, "message": "", "legacy": true}),
        json!({"snapshot": {}, "current": 0, "total": 0, "message": ""}),
        json!({"snapshot": {}, "current": 0, "total": 1, "message": "", "percent": 101}),
        json!({}),
        json!(null),
    ] {
        let error = workflow_run_progress_summary(&value)
            .expect_err("invalid persisted fields must remain visible");
        assert!(
            matches!(&error, WorkflowReadError::InvalidData(_)),
            "{value}: {error}"
        );
    }
}

#[test]
fn workflow_run_summary_keeps_input_fingerprint() {
    let fingerprint = json!({
        "workflow_mode": "shared_segment_plan",
        "options": { "style": "电影感写实" }
    });
    let run = WorkflowRunModelBuilder::new(
        1,
        "business",
        "video_segment_plan",
        WORKFLOW_STATUS_RUNNING,
        test_timestamp(),
    )
    .with_input_fingerprint(fingerprint.clone())
    .build();

    let summary = workflow_run_summary(run).expect("workflow run should project");

    assert_eq!(summary.input_fingerprint_json, Some(fingerprint));
}

#[test]
fn workflow_progress_write_range_rejects_zero_total_and_overrun() {
    assert!(validate_workflow_progress_range(0, 0).is_err());
    assert!(validate_workflow_progress_range(5, 4).is_err());
    validate_workflow_progress_range(4, 4).expect("valid progress range");
}

#[test]
fn validate_workflow_status_rejects_unknown_status() {
    assert!(validate_workflow_status("running").is_ok());
    assert_eq!(
        validate_workflow_status("paused")
            .expect_err("unknown status should fail")
            .to_string(),
        "不支持的工作流状态: paused"
    );
}

#[test]
fn persisted_task_info_from_row_maps_task_fields_and_workflow_id() {
    let task = persisted_task_info_from_row(task_row(
        "task-1",
        "running",
        None,
        Some(json!({
            "current": 2,
            "total": 5,
            "message": "排队中",
            "updated_at": "2026-01-02 03:04:05"
        })),
        Some(42),
    ))
    .expect("valid task row should map");

    assert_eq!(task.workflow_run_id, Some(42));
    assert_eq!(task.task.id, "task-1");
    assert_eq!(task.task.task_type, TaskKind::VideoGeneration);
    assert_eq!(task.task.priority, TaskPriority::High);
    assert!(matches!(task.task.status, TaskStatus::Running));
    assert_eq!(task.task.project_id, Some(7));
    let progress = task.task.progress.expect("progress should parse");
    assert_eq!(progress.current, 2);
    assert_eq!(progress.total, 5);
    assert_eq!(progress.message, "排队中");
    assert_eq!(progress.updated_at.as_deref(), Some("2026-01-02 03:04:05"));
}

#[test]
fn task_info_from_row_keeps_failed_task_error_requirement() {
    let error = task_info_from_row(task_row("task-2", "failed", None, None, None))
        .expect_err("failed task without error_message should fail");

    assert!(matches!(
        error,
        WorkflowReadError::InvalidData(message)
            if message == "参数无效：失败任务缺少 error_message"
    ));
}

#[test]
fn generated_video_ids_for_cancellation_checkpoints_keep_retry_targets() {
    let ids = generated_video_ids_for_cancellation_checkpoints(&[
        checkpoint_video_refs(
            WORKFLOW_STATUS_RUNNING,
            Some(json!({ "generated_video_id": 9 })),
            Some(json!({ "generated_video_id": 3 })),
        ),
        checkpoint_video_refs(
            WORKFLOW_STATUS_INTERRUPTED,
            Some(json!({ "generated_video_id": 3 })),
            None,
        ),
        checkpoint_video_refs(
            WORKFLOW_STATUS_COMPLETED,
            Some(json!({ "generated_video_id": 1 })),
            None,
        ),
        checkpoint_video_refs(
            WORKFLOW_STATUS_CANCELLED,
            Some(json!({ "generated_video_id": 7 })),
            None,
        ),
    ]);

    assert_eq!(ids, vec![3, 7, 9]);
}

#[test]
fn workflow_terminal_transition_requires_explicit_resume() {
    assert!(
        ensure_workflow_status_transition(
            WORKFLOW_STATUS_CANCELLED,
            WORKFLOW_STATUS_COMPLETED,
            false,
        )
        .is_err()
    );
    assert!(
        ensure_workflow_status_transition(
            WORKFLOW_STATUS_CANCELLED,
            WORKFLOW_STATUS_RUNNING,
            false,
        )
        .is_err()
    );
    ensure_workflow_status_transition(WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_RUNNING, true)
        .expect("显式恢复应允许 cancelled -> running");
    assert!(
        ensure_workflow_status_transition(
            WORKFLOW_STATUS_INTERRUPTED,
            WORKFLOW_STATUS_COMPLETED,
            false,
        )
        .is_err()
    );
    ensure_workflow_status_transition(WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, true)
        .expect("显式恢复应允许 interrupted -> running");
    assert!(
        ensure_workflow_status_transition(
            WORKFLOW_STATUS_COMPLETED,
            WORKFLOW_STATUS_RUNNING,
            true,
        )
        .is_err()
    );
}

#[test]
fn cancel_helpers_classify_terminal_status_and_require_reason() {
    assert!(is_terminal_workflow_status(WORKFLOW_STATUS_COMPLETED));
    assert!(is_terminal_workflow_status(WORKFLOW_STATUS_FAILED));
    assert!(!is_terminal_workflow_status(WORKFLOW_STATUS_RUNNING));
    assert!(required_text("  ".to_string(), "取消原因").is_err());
    assert_eq!(
        required_text(" 用户取消 ".to_string(), "取消原因").unwrap(),
        "用户取消"
    );
}

#[test]
fn workflow_write_helpers_keep_existing_status_semantics() {
    let running = workflow_run_row(WORKFLOW_STATUS_RUNNING);
    let completed = workflow_run_row(WORKFLOW_STATUS_COMPLETED);
    let interrupted = workflow_run_row(WORKFLOW_STATUS_INTERRUPTED);
    let mut failed = workflow_run_row(WORKFLOW_STATUS_FAILED);
    failed.can_resume = true;
    let mut cancelled = workflow_run_row(WORKFLOW_STATUS_CANCELLED);
    cancelled.can_resume = true;
    let mut resumable_interrupted = interrupted.clone();
    resumable_interrupted.can_resume = true;
    let timestamp = test_timestamp();

    assert!(workflow_run_is_running_or_resumable(
        &running.status,
        running.can_resume,
    ));
    assert!(workflow_run_is_running_or_resumable(
        &failed.status,
        failed.can_resume,
    ));
    assert!(workflow_run_is_running_or_resumable(
        &cancelled.status,
        cancelled.can_resume,
    ));
    assert!(!workflow_run_is_running_or_resumable(
        &completed.status,
        completed.can_resume,
    ));

    assert!(workflow_fingerprint_matches(
        Some(&json!({"chapter": 1})),
        Some(&json!({"chapter": 1}))
    ));
    assert!(should_update_existing_run(
        &running,
        WORKFLOW_STATUS_COMPLETED
    ));
    assert!(should_update_existing_run(
        &running,
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(!should_update_existing_run(
        &completed,
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(!should_update_existing_run(
        &workflow_run_row(WORKFLOW_STATUS_CANCELLED),
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(!should_update_existing_run(
        &interrupted,
        WORKFLOW_STATUS_RUNNING,
    ));
    assert!(!requires_explicit_interrupted_resume(
        &interrupted,
        WORKFLOW_STATUS_RUNNING,
    ));
    assert!(requires_explicit_interrupted_resume(
        &resumable_interrupted,
        WORKFLOW_STATUS_RUNNING,
    ));
    assert!(should_resume_existing_run(&failed, WORKFLOW_STATUS_RUNNING));
    assert!(should_resume_existing_run(
        &cancelled,
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(should_resume_existing_run(
        &resumable_interrupted,
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(!should_resume_existing_run(
        &completed,
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(!should_resume_existing_run(
        &running,
        WORKFLOW_STATUS_RUNNING
    ));
    assert!(completed_at_for_status_value(WORKFLOW_STATUS_COMPLETED, timestamp).is_some());
    assert!(completed_at_for_status_value(WORKFLOW_STATUS_INTERRUPTED, timestamp).is_some());
    assert!(completed_at_for_status_value(WORKFLOW_STATUS_RUNNING, timestamp).is_none());
}

#[test]
fn validate_workflow_run_input_rejects_invalid_ids() {
    let mut input = workflow_run_input();
    input.project_id = Some(0);

    assert_eq!(
        validate_workflow_run_input(&input)
            .expect_err("invalid project id should fail")
            .to_string(),
        "project_id 必须大于 0"
    );
}

#[test]
fn validate_workflow_run_input_requires_terminal_reasons() {
    let mut input = workflow_run_input();
    input.status = WORKFLOW_STATUS_COMPLETED.to_string();
    input.can_resume = true;
    assert_eq!(
        validate_workflow_run_input(&input)
            .expect_err("completed workflow must not be resumable")
            .to_string(),
        "completed 工作流不能标记为可恢复"
    );

    input.status = WORKFLOW_STATUS_FAILED.to_string();
    assert_eq!(
        validate_workflow_run_input(&input)
            .expect_err("failed workflow without error should fail")
            .to_string(),
        "failed 工作流必须提供 error_message"
    );

    input.error_message = Some("provider failed".to_string());
    assert!(validate_workflow_run_input(&input).is_ok());

    input.can_resume = false;
    assert_eq!(
        validate_workflow_run_input(&input)
            .expect_err("non-resumable failed workflow without block reason should fail")
            .to_string(),
        "不可恢复的 failed 工作流必须提供 resume_block_reason"
    );

    input.resume_block_reason = Some("输入已失效".to_string());
    assert!(validate_workflow_run_input(&input).is_ok());
}

fn task_row(
    id: &str,
    status: &str,
    error_message: Option<&str>,
    progress: Option<Value>,
    workflow_run_id: Option<i64>,
) -> task_queue::Model {
    let timestamp = test_timestamp();
    task_queue::Model {
        id: id.to_string(),
        task_type: "video_generation".to_string(),
        priority: "high".to_string(),
        status: status.to_string(),
        project_id: Some(7),
        root_project_id: Some(3),
        source_snapshot_id: Some(11),
        task_scope: Some("media_generation".to_string()),
        workflow_run_id,
        payload: None,
        progress,
        error_message: error_message.map(ToString::to_string),
        dag_id: None,
        created_at: timestamp,
        started_at: Some(timestamp),
        completed_at: None,
    }
}

fn checkpoint_video_refs(
    status: &str,
    artifacts_json: Option<Value>,
    input_fingerprint_json: Option<Value>,
) -> WorkflowCheckpointVideoRefs {
    WorkflowCheckpointVideoRefs {
        status: status.to_string(),
        artifact_generated_video_id: artifacts_json
            .and_then(|value| value.get("generated_video_id").cloned()),
        input_generated_video_id: input_fingerprint_json
            .and_then(|value| value.get("generated_video_id").cloned()),
    }
}

fn workflow_run_input() -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: "business".to_string(),
        workflow_kind: "test_workflow".to_string(),
        project_id: Some(1),
        root_project_id: Some(1),
        source_snapshot_id: None,
        scope_table: None,
        scope_id: None,
        chapter_id: None,
        mode: None,
        input_fingerprint_json: None,
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: None,
        active_task_id: None,
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    }
}

fn workflow_run_row(status: &str) -> workflow_run::Model {
    let timestamp = test_timestamp();
    WorkflowRunModelBuilder::new(1, "business", "test_workflow", status, timestamp)
        .with_project_scope(Some(1), Some(1))
        .build()
}

fn test_timestamp() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 2)
        .expect("valid date")
        .and_hms_opt(3, 4, 5)
        .expect("valid time")
}

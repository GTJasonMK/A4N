//! 导入反推结果的持久化候选与确认应用。
//!
//! 反推的 LLM 结果已经保存在 workflow checkpoint；这里把该 checkpoint 明确投影为
//! 候选，并在用户确认时从同一 checkpoint 读取、一次事务写入正式资料。确认阶段不再
//! 接受调用方上传的 JSON，也不重新调用模型。

use a4n_core::{
    error::AppError,
    import_reverse::ReverseContentRequest,
    models::generation::{workflow_checkpoint, workflow_run},
};
use a4n_db::{
    import_reverse::{
        apply_reverse_blueprint_records, apply_reverse_outline_records,
        apply_reverse_part_outline_records, reverse_context_rows,
    },
    project::{lock_project_chapter_rows, project_for_update},
    workflow::WORKFLOW_STATUS_COMPLETED,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, Set, TransactionTrait,
};
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::{
    ReverseEngineeringTarget,
    application::{
        reverse_blueprint_apply_draft, reverse_blueprint_apply_result,
        reverse_outline_apply_result, reverse_outline_rows, reverse_part_outline_apply_result,
        reverse_part_outline_rows, validate_reverse_engineer_output,
    },
    execution::restored_import_reverse_checkpoint,
    import_reverse_db_error,
};

pub(crate) const REVERSE_CANDIDATE_ARTIFACT_KEY: &str = "candidate";

/// 在 LLM checkpoint 中保存一个可审阅的候选标记。
pub(crate) fn reverse_candidate_proposal(
    workflow_run_id: i64,
    prompt_name: &str,
    output_fingerprint_sha256: &str,
) -> Value {
    json!({
        "candidate_id": workflow_run_id,
        "workflow_run_id": workflow_run_id,
        "origin": "import_reverse",
        "target": prompt_name,
        "state": "proposal",
        "output_fingerprint_sha256": output_fingerprint_sha256,
    })
}

pub(crate) fn reverse_target_from_prompt_name(
    prompt_name: &str,
) -> Result<ReverseEngineeringTarget, AppError> {
    ReverseEngineeringTarget::from_prompt_name(prompt_name).ok_or_else(|| AppError::InvalidState {
        current: prompt_name.to_string(),
        operation: "读取导入反推候选目标".to_string(),
    })
}

pub(crate) fn reverse_candidate_response(
    workflow_run_id: i64,
    target: ReverseEngineeringTarget,
    state: &str,
    result: Option<Value>,
    applied: Option<Value>,
    warnings: Vec<String>,
) -> Value {
    let mut response = json!({
        "candidate_id": workflow_run_id,
        "workflow_run_id": workflow_run_id,
        "origin": "import_reverse",
        "target": target.prompt_name(),
        "state": state,
        "warnings": warnings,
    });
    if let Some(result) = result {
        response["result"] = result;
    }
    if let Some(applied) = applied {
        response["applied"] = applied;
    }
    response
}

pub(crate) async fn confirm_reverse_engineering_candidate(
    context: &OrchestratorContext,
    workflow_run_id: i64,
) -> Result<Value, AppError> {
    if workflow_run_id <= 0 {
        return Err(AppError::InvalidArgument(
            "workflow_run_id 必须大于 0".to_string(),
        ));
    }

    let tx = context.database.connection().begin().await?;

    // 反推样本依赖正文；项目行先串行化正式写入，再锁正文与 workflow run。
    let run_hint = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("工作流不存在: {workflow_run_id}")))?;
    let project_id =
        run_hint
            .project_id
            .filter(|id| *id > 0)
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_project_id".to_string(),
                operation: "确认导入反推候选".to_string(),
            })?;
    let project = project_for_update(&tx, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if let Some(operation) = a4n_db::project::blocked_project_write_operation(
        &project.lifecycle_gate,
        "确认导入反推候选",
    ) {
        return Err(AppError::InvalidState {
            current: project.lifecycle_gate,
            operation,
        });
    }
    lock_project_chapter_rows(&tx, project_id).await?;
    let mut run_query = workflow_run::Entity::find_by_id(workflow_run_id);
    if tx.get_database_backend() != DbBackend::Sqlite {
        run_query = run_query.lock_exclusive();
    }
    let run = run_query
        .one(&tx)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("工作流不存在: {workflow_run_id}")))?;

    let prompt_name = run
        .current_step_key
        .as_deref()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate_step".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    let target = reverse_target_from_prompt_name(prompt_name)?;
    validate_reverse_candidate_run(&run, project_id, prompt_name)?;
    let mut checkpoint_query = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(prompt_name));
    if tx.get_database_backend() != DbBackend::Sqlite {
        checkpoint_query = checkpoint_query.lock_exclusive();
    }
    let checkpoint = checkpoint_query.one(&tx).await?.ok_or_else(|| {
        AppError::NotFound(format!(
            "工作流 {workflow_run_id} 缺少反推 checkpoint {prompt_name}"
        ))
    })?;
    validate_reverse_candidate_checkpoint(&checkpoint, workflow_run_id, prompt_name)?;

    let checkpoint_json =
        checkpoint
            .checkpoint_json
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_checkpoint_json".to_string(),
                operation: "确认导入反推候选".to_string(),
            })?;
    let parsed_output = restored_import_reverse_checkpoint(checkpoint_json, prompt_name)?;
    validate_reverse_engineer_output(prompt_name, &parsed_output)?;

    let mut artifacts = checkpoint
        .artifacts_json
        .clone()
        .unwrap_or_else(|| json!({}));
    let artifacts_object = artifacts
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate_artifacts".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    let mut candidate = artifacts_object
        .get(REVERSE_CANDIDATE_ARTIFACT_KEY)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    let candidate_object = candidate
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    validate_candidate_identity(
        candidate_object,
        workflow_run_id,
        prompt_name,
        &parsed_output,
    )?;

    if let Some(applied) = confirmed_reverse_candidate_apply_result(candidate_object)? {
        tx.rollback().await?;
        // 正式资料可能已提交但状态推进在响应前失败；重试应只补状态，不重写资料。
        let warnings = project_status_warnings(
            context
                .advance_project_status_if_blueprint_exists(project_id, target.applied_status())
                .await
                .err()
                .map(|error| error.to_string()),
        );
        return Ok(reverse_candidate_response(
            workflow_run_id,
            target,
            "already_confirmed",
            None,
            Some(applied),
            warnings,
        ));
    }

    validate_reverse_candidate_source_freshness(
        &tx,
        project_id,
        target,
        run.input_fingerprint_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_reverse_candidate_input_fingerprint".to_string(),
                operation: "确认导入反推候选".to_string(),
            })?,
    )
    .await?;

    let applied = match target {
        ReverseEngineeringTarget::Blueprint => {
            let draft =
                reverse_blueprint_apply_draft(&parsed_output).map_err(AppError::InvalidArgument)?;
            let result = apply_reverse_blueprint_records(&tx, project_id, draft)
                .await
                .map_err(import_reverse_db_error)?;
            reverse_blueprint_apply_result(result)
        }
        ReverseEngineeringTarget::Outline => {
            let rows = reverse_outline_rows(&parsed_output).map_err(AppError::InvalidArgument)?;
            let count = apply_reverse_outline_records(&tx, project_id, &rows)
                .await
                .map_err(import_reverse_db_error)?;
            reverse_outline_apply_result(count)
        }
        ReverseEngineeringTarget::PartOutline => {
            let rows =
                reverse_part_outline_rows(&parsed_output).map_err(AppError::InvalidArgument)?;
            let result = apply_reverse_part_outline_records(&tx, project_id, &rows)
                .await
                .map_err(import_reverse_db_error)?;
            reverse_part_outline_apply_result(result)
        }
    };

    let now = Utc::now().naive_utc();
    candidate_object.insert("state".to_string(), json!("confirmed"));
    candidate_object.insert("confirmed_at".to_string(), json!(now.to_string()));
    candidate_object.insert("apply_result".to_string(), applied.clone());
    artifacts_object.insert(REVERSE_CANDIDATE_ARTIFACT_KEY.to_string(), candidate);
    let mut checkpoint_active: workflow_checkpoint::ActiveModel = checkpoint.into();
    checkpoint_active.artifacts_json = Set(Some(artifacts));
    checkpoint_active.updated_at = Set(now);
    checkpoint_active
        .update(&tx)
        .await
        .map_err(AppError::Database)?;
    tx.commit().await?;

    let warnings = project_status_warnings(
        context
            .advance_project_status_if_blueprint_exists(project_id, target.applied_status())
            .await
            .err()
            .map(|error| error.to_string()),
    );
    Ok(reverse_candidate_response(
        workflow_run_id,
        target,
        "confirmed",
        None,
        Some(applied),
        warnings,
    ))
}

fn project_status_warnings(error: Option<String>) -> Vec<String> {
    error
        .map(|error| format!("项目进度状态更新失败：{error}。正式资料已写入，可重试确认恢复状态"))
        .into_iter()
        .collect()
}

fn validate_reverse_candidate_run(
    run: &workflow_run::Model,
    project_id: i64,
    prompt_name: &str,
) -> Result<(), AppError> {
    if run.workflow_kind != "import_reverse_engineer" {
        return Err(AppError::InvalidArgument(
            "workflow_run_id 不是导入反推候选".to_string(),
        ));
    }
    if run.mode.as_deref() != Some("reverse") {
        return Err(AppError::InvalidArgument(
            "只能确认 reverse 模式的导入反推候选".to_string(),
        ));
    }
    if run.project_id != Some(project_id) || run.root_project_id != Some(project_id) {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_project_scope_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    if run.status != WORKFLOW_STATUS_COMPLETED {
        return Err(AppError::InvalidState {
            current: run.status.clone(),
            operation: "确认导入反推候选（工作流必须已完成）".to_string(),
        });
    }
    if run.current_step_key.as_deref() != Some(prompt_name) {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_step_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    let fingerprint =
        run.input_fingerprint_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_reverse_candidate_input_fingerprint".to_string(),
                operation: "确认导入反推候选".to_string(),
            })?;
    validate_reverse_candidate_run_fingerprint(fingerprint, project_id, prompt_name)?;
    let expected_business_key =
        crate::import_reverse::import_reverse_business_key(project_id, prompt_name)
            .map_err(AppError::InvalidArgument)?;
    if run.business_key != expected_business_key {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_business_key_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    Ok(())
}

fn validate_reverse_candidate_run_fingerprint(
    fingerprint: &Value,
    project_id: i64,
    prompt_name: &str,
) -> Result<(), AppError> {
    let expected = serde_json::json!({
        "schema_version": 1,
        "workflow": "import_reverse_engineer",
        "target": prompt_name,
        "mode": "reverse",
        "project_id": project_id,
    });
    for key in ["schema_version", "workflow", "target", "mode", "project_id"] {
        if fingerprint.get(key) != expected.get(key) {
            return Err(AppError::InvalidState {
                current: format!("reverse_candidate_input_fingerprint_{key}_mismatch"),
                operation: "确认导入反推候选".to_string(),
            });
        }
    }
    fingerprint
        .get("content_sample_sha256")
        .and_then(Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate_content_sample_fingerprint".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    let valid_context_limit = match fingerprint.get("context_limit") {
        Some(Value::Null) => true,
        Some(Value::Number(value)) => value.as_i64().is_some_and(|value| value >= 0),
        _ => false,
    };
    if fingerprint
        .get("auto_context")
        .and_then(Value::as_bool)
        .is_none()
        || !valid_context_limit
        || !fingerprint.get("variables").is_some_and(Value::is_object)
    {
        return Err(AppError::InvalidState {
            current: "invalid_reverse_candidate_input_fingerprint_shape".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    if fingerprint
        .get("auto_context")
        .and_then(Value::as_bool)
        .is_some_and(|value| value)
        && !valid_sha256(fingerprint.get("auto_context_source_sha256"))
    {
        return Err(AppError::InvalidState {
            current: "invalid_reverse_candidate_auto_context_fingerprint".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    Ok(())
}

async fn validate_reverse_candidate_source_freshness(
    db: &sea_orm::DatabaseTransaction,
    project_id: i64,
    target: ReverseEngineeringTarget,
    fingerprint: &Value,
) -> Result<(), AppError> {
    if !fingerprint
        .get("auto_context")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(());
    }

    let expected_hash = fingerprint
        .get("auto_context_source_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate_auto_context_fingerprint".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    let context_limit = fingerprint
        .get("context_limit")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate_context_limit".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    let rows = reverse_context_rows(
        db,
        project_id,
        matches!(target, ReverseEngineeringTarget::Blueprint),
    )
    .await
    .map_err(import_reverse_db_error)?;
    let (_, current_hash) = super::context::resolve_reverse_content_sample_with_provenance(
        ReverseContentRequest {
            project_id,
            manual_sample: "",
            auto_context: true,
            context_limit: Some(context_limit),
        },
        target.context_kind(),
        Some(&rows),
    )
    .map_err(AppError::InvalidArgument)?;
    let current_hash = current_hash.ok_or_else(|| AppError::InvalidState {
        current: "missing_reverse_candidate_current_auto_context_fingerprint".to_string(),
        operation: "确认导入反推候选".to_string(),
    })?;
    validate_auto_context_source_hash(expected_hash, &current_hash)
}

fn validate_auto_context_source_hash(expected: &str, current: &str) -> Result<(), AppError> {
    if expected == current {
        return Ok(());
    }
    Err(AppError::InvalidState {
        current: "reverse_candidate_source_changed".to_string(),
        operation: "确认导入反推候选（项目上下文已变化，请重新生成候选）".to_string(),
    })
}

fn valid_sha256(value: Option<&Value>) -> bool {
    value.and_then(Value::as_str).is_some_and(|value| {
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn validate_reverse_candidate_checkpoint(
    checkpoint: &workflow_checkpoint::Model,
    workflow_run_id: i64,
    prompt_name: &str,
) -> Result<(), AppError> {
    if checkpoint.workflow_run_id != workflow_run_id
        || checkpoint.step_key != prompt_name
        || checkpoint.step_order != 1
    {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_checkpoint_identity_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    if checkpoint.status != WORKFLOW_STATUS_COMPLETED {
        return Err(AppError::InvalidState {
            current: checkpoint.status.clone(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    let fingerprint =
        checkpoint
            .input_fingerprint_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_reverse_candidate_checkpoint_fingerprint".to_string(),
                operation: "确认导入反推候选".to_string(),
            })?;
    let expected = serde_json::json!({
        "schema_version": 1,
        "workflow": "import_reverse_engineer",
        "call_key": prompt_name,
        "prompt_name": prompt_name,
    });
    for key in ["schema_version", "workflow", "call_key", "prompt_name"] {
        if fingerprint.get(key) != expected.get(key) {
            return Err(AppError::InvalidState {
                current: format!("reverse_candidate_checkpoint_{key}_mismatch"),
                operation: "确认导入反推候选".to_string(),
            });
        }
    }
    let expected_hash = a4n_db::workflow::llm_call_checkpoint_fingerprint_sha256(fingerprint)
        .map_err(|error| AppError::InvalidState {
            current: format!("invalid_reverse_candidate_checkpoint_fingerprint: {error}"),
            operation: "确认导入反推候选".to_string(),
        })?;
    if checkpoint.input_fingerprint_sha256.as_deref() != Some(expected_hash.as_str()) {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_checkpoint_fingerprint_hash_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    Ok(())
}

fn validate_candidate_identity(
    candidate: &serde_json::Map<String, Value>,
    workflow_run_id: i64,
    prompt_name: &str,
    parsed_output: &Value,
) -> Result<(), AppError> {
    if candidate.get("candidate_id").and_then(Value::as_i64) != Some(workflow_run_id)
        || candidate.get("workflow_run_id").and_then(Value::as_i64) != Some(workflow_run_id)
    {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_id_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    if candidate.get("origin").and_then(Value::as_str) != Some("import_reverse")
        || candidate.get("target").and_then(Value::as_str) != Some(prompt_name)
    {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_target_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    let output_fingerprint = candidate
        .get("output_fingerprint_sha256")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate_output_fingerprint".to_string(),
            operation: "确认导入反推候选".to_string(),
        })?;
    let actual = a4n_db::generation::generation_checkpoint_fingerprint_sha256(parsed_output)
        .map_err(AppError::Serialization)?;
    if output_fingerprint != actual {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_output_fingerprint_mismatch".to_string(),
            operation: "确认导入反推候选".to_string(),
        });
    }
    Ok(())
}

fn confirmed_reverse_candidate_apply_result(
    candidate: &serde_json::Map<String, Value>,
) -> Result<Option<Value>, AppError> {
    match candidate.get("state").and_then(Value::as_str) {
        Some("proposal") => Ok(None),
        Some("confirmed") => candidate
            .get("apply_result")
            .filter(|value| value.is_object())
            .cloned()
            .map(Some)
            .ok_or_else(|| AppError::InvalidState {
                current: "confirmed_reverse_candidate_missing_result".to_string(),
                operation: "读取已确认导入反推候选".to_string(),
            }),
        Some(other) => Err(AppError::InvalidState {
            current: other.to_string(),
            operation: "确认导入反推候选".to_string(),
        }),
        None => Err(AppError::InvalidState {
            current: "missing_reverse_candidate_state".to_string(),
            operation: "确认导入反推候选".to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp() -> chrono::NaiveDateTime {
        chrono::NaiveDate::from_ymd_opt(2026, 8, 8)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
    }

    fn completed_run() -> workflow_run::Model {
        workflow_run::Model {
            id: 42,
            business_key: "import_reverse_engineer:project:7:reverse_outline:reverse".to_string(),
            workflow_kind: "import_reverse_engineer".to_string(),
            project_id: Some(7),
            root_project_id: Some(7),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(7),
            chapter_id: None,
            mode: Some("reverse".to_string()),
            input_fingerprint_json: Some(json!({
                "schema_version": 1,
                "workflow": "import_reverse_engineer",
                "target": "reverse_outline",
                "mode": "reverse",
                "project_id": 7,
                "content_sample_sha256": "a".repeat(64),
                "auto_context": true,
                "context_limit": 8,
                "auto_context_source_sha256": "b".repeat(64),
                "variables": {},
            })),
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            current_step_key: Some("reverse_outline".to_string()),
            progress_json: None,
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
            created_at: timestamp(),
            updated_at: timestamp(),
            completed_at: Some(timestamp()),
        }
    }

    fn completed_checkpoint() -> workflow_checkpoint::Model {
        let fingerprint = json!({
            "schema_version": 1,
            "workflow": "import_reverse_engineer",
            "call_key": "reverse_outline",
            "prompt_name": "reverse_outline",
            "messages": [],
            "context_manifest": null,
            "max_tokens": 4096,
            "max_tokens_profile": "outline",
            "response_format": null,
        });
        let hash = a4n_db::workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint).unwrap();
        workflow_checkpoint::Model {
            id: 9,
            workflow_run_id: 42,
            step_key: "reverse_outline".to_string(),
            step_order: 1,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some(hash),
            input_fingerprint_json: Some(fingerprint),
            checkpoint_json: None,
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            created_at: timestamp(),
            updated_at: timestamp(),
            completed_at: Some(timestamp()),
        }
    }

    fn invalid_state(error: AppError) -> String {
        match error {
            AppError::InvalidState { current, .. } => current,
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn candidate_proposal_is_stable_and_identifiable() {
        let proposal = reverse_candidate_proposal(42, "reverse_outline", "output-hash");
        assert_eq!(proposal["candidate_id"], 42);
        assert_eq!(proposal["workflow_run_id"], 42);
        assert_eq!(proposal["state"], "proposal");
        assert_eq!(proposal["output_fingerprint_sha256"], "output-hash");
        assert_eq!(
            reverse_target_from_prompt_name("reverse_part_outline").unwrap(),
            ReverseEngineeringTarget::PartOutline
        );
    }

    #[test]
    fn post_commit_status_failure_is_returned_as_a_warning() {
        let response = reverse_candidate_response(
            42,
            ReverseEngineeringTarget::Outline,
            "confirmed",
            None,
            Some(json!({"outline_count": 3})),
            project_status_warnings(Some("状态服务不可用".to_string())),
        );

        assert_eq!(response["state"], "confirmed");
        assert_eq!(response["applied"]["outline_count"], 3);
        assert!(
            response["warnings"][0]
                .as_str()
                .is_some_and(|warning| warning.contains("正式资料已写入"))
        );
    }

    #[test]
    fn unknown_candidate_target_is_rejected() {
        assert!(reverse_target_from_prompt_name("reverse_unknown").is_err());
    }

    #[test]
    fn unfinished_workflow_cannot_be_confirmed() {
        let mut run = completed_run();
        run.status = "running".to_string();

        let error = validate_reverse_candidate_run(&run, 7, "reverse_outline").unwrap_err();

        assert_eq!(invalid_state(error), "running");
    }

    #[test]
    fn workflow_fingerprint_tampering_is_rejected() {
        let mut run = completed_run();
        run.input_fingerprint_json.as_mut().unwrap()["workflow"] = json!("other_workflow");

        let error = validate_reverse_candidate_run(&run, 7, "reverse_outline").unwrap_err();

        assert_eq!(
            invalid_state(error),
            "reverse_candidate_input_fingerprint_workflow_mismatch"
        );
    }

    #[test]
    fn malformed_workflow_source_fingerprint_is_rejected() {
        let mut run = completed_run();
        run.input_fingerprint_json.as_mut().unwrap()["content_sample_sha256"] = json!("short");

        let error = validate_reverse_candidate_run(&run, 7, "reverse_outline").unwrap_err();

        assert_eq!(
            invalid_state(error),
            "invalid_reverse_candidate_content_sample_fingerprint"
        );
    }

    #[test]
    fn checkpoint_fingerprint_tampering_is_rejected() {
        let mut checkpoint = completed_checkpoint();
        checkpoint.input_fingerprint_json.as_mut().unwrap()["messages"] = json!([{"role": "user"}]);

        let error =
            validate_reverse_candidate_checkpoint(&checkpoint, 42, "reverse_outline").unwrap_err();

        assert_eq!(
            invalid_state(error),
            "reverse_candidate_checkpoint_fingerprint_hash_mismatch"
        );
    }

    #[test]
    fn checkpoint_target_cannot_be_changed_with_a_recomputed_hash() {
        let mut checkpoint = completed_checkpoint();
        let fingerprint = checkpoint.input_fingerprint_json.as_mut().unwrap();
        fingerprint["call_key"] = json!("reverse_blueprint");
        checkpoint.input_fingerprint_sha256 =
            Some(a4n_db::workflow::llm_call_checkpoint_fingerprint_sha256(fingerprint).unwrap());

        let error =
            validate_reverse_candidate_checkpoint(&checkpoint, 42, "reverse_outline").unwrap_err();

        assert_eq!(
            invalid_state(error),
            "reverse_candidate_checkpoint_call_key_mismatch"
        );
    }

    #[test]
    fn candidate_identity_target_and_output_tampering_are_rejected() {
        let output = json!({"outlines": []});
        let output_hash =
            a4n_db::generation::generation_checkpoint_fingerprint_sha256(&output).unwrap();
        let candidate = reverse_candidate_proposal(42, "reverse_outline", &output_hash);
        validate_candidate_identity(
            candidate.as_object().unwrap(),
            42,
            "reverse_outline",
            &output,
        )
        .unwrap();

        for (field, value, expected) in [
            ("candidate_id", json!(43), "reverse_candidate_id_mismatch"),
            (
                "target",
                json!("reverse_blueprint"),
                "reverse_candidate_target_mismatch",
            ),
            (
                "output_fingerprint_sha256",
                json!("tampered"),
                "reverse_candidate_output_fingerprint_mismatch",
            ),
        ] {
            let mut tampered = candidate.clone();
            tampered[field] = value;
            let error = validate_candidate_identity(
                tampered.as_object().unwrap(),
                42,
                "reverse_outline",
                &output,
            )
            .unwrap_err();
            assert_eq!(invalid_state(error), expected);
        }
    }

    #[test]
    fn confirmed_candidate_returns_stored_result_without_reapplying() {
        let mut candidate = reverse_candidate_proposal(42, "reverse_outline", "output-hash");
        candidate["state"] = json!("confirmed");
        candidate["apply_result"] = json!({"outline_count": 3});

        let applied = confirmed_reverse_candidate_apply_result(candidate.as_object().unwrap())
            .unwrap()
            .expect("confirmed candidate must take the idempotent return path");

        assert_eq!(applied, json!({"outline_count": 3}));
        assert!(
            confirmed_reverse_candidate_apply_result(
                reverse_candidate_proposal(42, "reverse_outline", "output-hash")
                    .as_object()
                    .unwrap()
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn confirmed_candidate_requires_structured_apply_result() {
        let mut candidate = reverse_candidate_proposal(42, "reverse_outline", "output-hash");
        candidate["state"] = json!("confirmed");
        candidate["apply_result"] = json!(3);

        let error =
            confirmed_reverse_candidate_apply_result(candidate.as_object().unwrap()).unwrap_err();

        assert_eq!(
            invalid_state(error),
            "confirmed_reverse_candidate_missing_result"
        );
    }

    #[test]
    fn unchanged_auto_context_source_can_be_confirmed() {
        assert!(validate_auto_context_source_hash("a", "a").is_ok());
    }

    #[test]
    fn changed_auto_context_source_requires_regeneration() {
        let error = validate_auto_context_source_hash("a", "b").unwrap_err();
        assert_eq!(invalid_state(error), "reverse_candidate_source_changed");
    }

    #[test]
    fn manual_sample_does_not_require_auto_context_freshness() {
        let mut run = completed_run();
        let fingerprint = run.input_fingerprint_json.as_mut().unwrap();
        fingerprint["auto_context"] = json!(false);
        fingerprint["auto_context_source_sha256"] = Value::Null;
        validate_reverse_candidate_run(&run, 7, "reverse_outline")
            .expect("manual samples use immutable checkpoint text only");
    }
}

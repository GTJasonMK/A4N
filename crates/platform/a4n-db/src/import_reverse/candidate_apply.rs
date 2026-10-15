use a4n_db_models::models::generation::{workflow_checkpoint, workflow_run};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde_json::{Value, json};

use a4n_domain::{
    import_reverse::{WORKFLOW_KIND, WORKFLOW_MODE, business_key},
    project::ProjectStatus,
    workflow::WORKFLOW_STATUS_COMPLETED,
};

use crate::{
    generation::PROJECT_GATE_ACTIVE,
    outline::load_outline_generation_source_rows_for_update,
    project::{ProjectStatusWriteError, update_project_status_record_in},
    workflow::completed_llm_call_checkpoint_from_model,
};

use super::{
    ImportReverseDbError, ReverseBlueprintApplyResult, ReverseCandidateApplyData,
    ReverseCandidateApplyInput, ReverseCandidateApplyResult, ReverseCandidateProjectStatusChange,
    ReverseContextRows,
};

pub async fn apply_reverse_candidate_records<C, F>(
    db: &C,
    input: ReverseCandidateApplyInput,
    validate_source_freshness: F,
) -> Result<ReverseCandidateApplyResult, ImportReverseDbError>
where
    C: ConnectionTrait + TransactionTrait,
    F: Fn(&ReverseContextRows) -> Result<bool, String>,
{
    validate_apply_target(&input)?;
    if input.workflow_run_id <= 0 || input.project_id <= 0 {
        return Err(ImportReverseDbError::InvalidArgument(
            "workflow_run_id/project_id 必须大于 0".to_string(),
        ));
    }

    let txn = db.begin().await?;
    let run_hint = workflow_run::Entity::find_by_id(input.workflow_run_id)
        .one(&txn)
        .await?
        .ok_or_else(|| {
            ImportReverseDbError::NotFound(format!("工作流不存在: {}", input.workflow_run_id))
        })?;
    let project_id = run_hint.project_id.filter(|id| *id > 0).ok_or_else(|| {
        ImportReverseDbError::InvalidState {
            current: "missing_project_id".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        }
    })?;
    if project_id != input.project_id {
        return Err(ImportReverseDbError::InvalidState {
            current: "reverse_candidate_project_scope_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }

    let locked_sources = load_outline_generation_source_rows_for_update(&txn, project_id)
        .await?
        .ok_or_else(|| ImportReverseDbError::NotFound(format!("项目不存在: {project_id}")))?;
    if locked_sources.project.lifecycle_gate != PROJECT_GATE_ACTIVE {
        return Err(ImportReverseDbError::InvalidState {
            current: locked_sources.project.lifecycle_gate,
            operation: "写入项目资料归纳结果".to_string(),
        });
    }

    let mut run_query = workflow_run::Entity::find_by_id(input.workflow_run_id);
    if txn.get_database_backend() != DbBackend::Sqlite {
        run_query = run_query.lock_exclusive();
    }
    let run = run_query.one(&txn).await?.ok_or_else(|| {
        ImportReverseDbError::NotFound(format!("工作流不存在: {}", input.workflow_run_id))
    })?;
    validate_run_snapshot(&run, &input)?;

    let mut checkpoint_query = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(input.workflow_run_id))
        .filter(workflow_checkpoint::Column::StepKey.eq(input.target.clone()));
    if txn.get_database_backend() != DbBackend::Sqlite {
        checkpoint_query = checkpoint_query.lock_exclusive();
    }
    let checkpoint = checkpoint_query.one(&txn).await?.ok_or_else(|| {
        ImportReverseDbError::NotFound(format!(
            "工作流 {} 缺少反推 checkpoint {}",
            input.workflow_run_id, input.target
        ))
    })?;
    validate_checkpoint_snapshot(&checkpoint, &input)?;
    completed_llm_call_checkpoint_from_model(&checkpoint, &input.target).map_err(|error| {
        ImportReverseDbError::InvalidState {
            current: format!("reverse_candidate_checkpoint_invalid: {error}"),
            operation: "写入项目资料归纳结果".to_string(),
        }
    })?;
    validate_candidate_output_fingerprint(&checkpoint, &input.output)?;

    let mut artifacts =
        checkpoint
            .artifacts_json
            .clone()
            .ok_or_else(|| ImportReverseDbError::InvalidState {
                current: "missing_reverse_candidate_artifacts".to_string(),
                operation: "写入项目资料归纳结果".to_string(),
            })?;
    let artifacts_object =
        artifacts
            .as_object_mut()
            .ok_or_else(|| ImportReverseDbError::InvalidState {
                current: "invalid_reverse_candidate_artifacts".to_string(),
                operation: "写入项目资料归纳结果".to_string(),
            })?;
    let mut candidate = artifacts_object.get("candidate").cloned().ok_or_else(|| {
        ImportReverseDbError::InvalidState {
            current: "missing_reverse_candidate".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        }
    })?;
    let candidate_object =
        candidate
            .as_object_mut()
            .ok_or_else(|| ImportReverseDbError::InvalidState {
                current: "invalid_reverse_candidate".to_string(),
                operation: "写入项目资料归纳结果".to_string(),
            })?;

    if let Some(applied) = confirmed_candidate_result(candidate_object)? {
        let current_status = ProjectStatus::parse_storage(&locked_sources.project.status)
            .map_err(|error| ImportReverseDbError::InvalidArgument(error.to_string()))?;
        let target_status = applied_status(&input.target)?;
        if current_status < target_status {
            return Err(ImportReverseDbError::InvalidState {
                current: current_status.as_str().to_string(),
                operation: format!(
                    "读取已确认的项目资料归纳结果（期望项目状态至少为 {}）",
                    target_status.as_str()
                ),
            });
        }
        if input.finalize_running_workflow {
            crate::workflow::mark_workflow_status_in_transaction(
                &txn,
                run,
                WORKFLOW_STATUS_COMPLETED,
                false,
                None,
            )
            .await?;
        }
        txn.commit().await?;
        return Ok(ReverseCandidateApplyResult {
            apply_result: applied,
            status_change: None,
        });
    }

    if input
        .run
        .input_fingerprint_json
        .as_ref()
        .and_then(|fingerprint| fingerprint.get("auto_context"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let rows = super::context::reverse_context_rows(
            &txn,
            project_id,
            input.target == "reverse_blueprint",
        )
        .await?;
        match validate_source_freshness(&rows) {
            Ok(true) => {}
            Ok(false) => {
                return Err(ImportReverseDbError::InvalidState {
                    current: "reverse_candidate_source_changed".to_string(),
                    operation: "写入项目资料归纳结果（项目上下文已变化，请重新归纳）".to_string(),
                });
            }
            Err(message) => return Err(ImportReverseDbError::InvalidArgument(message)),
        }
    }

    let applied = apply_candidate_data(&txn, project_id, input.apply_data).await?;
    let now = Utc::now().naive_utc();
    let current_status = ProjectStatus::parse_storage(&locked_sources.project.status)
        .map_err(|error| ImportReverseDbError::InvalidArgument(error.to_string()))?;
    let target_status = applied_status(&input.target)?;
    let status_change = if current_status < target_status {
        update_project_status_record_in(&txn, locked_sources.project, target_status, now)
            .await
            .map_err(project_status_write_error)?;
        Some(ReverseCandidateProjectStatusChange {
            from: current_status,
            to: target_status,
            changed_at: now,
        })
    } else {
        None
    };

    candidate_object.insert("state".to_string(), Value::String("confirmed".to_string()));
    candidate_object.insert("apply_result".to_string(), applied.clone());
    artifacts_object.insert("candidate".to_string(), candidate);
    let mut checkpoint_active: workflow_checkpoint::ActiveModel = checkpoint.into();
    checkpoint_active.artifacts_json = Set(Some(artifacts));
    checkpoint_active.updated_at = Set(now);
    checkpoint_active.update(&txn).await?;
    if input.finalize_running_workflow {
        // Blueprint material and run completion commit together so cancellation cannot follow the write.
        crate::workflow::mark_workflow_status_in_transaction(
            &txn,
            run,
            WORKFLOW_STATUS_COMPLETED,
            false,
            None,
        )
        .await?;
    }
    txn.commit().await?;

    Ok(ReverseCandidateApplyResult {
        apply_result: applied,
        status_change,
    })
}

async fn apply_candidate_data(
    txn: &DatabaseTransaction,
    project_id: i64,
    input: ReverseCandidateApplyData,
) -> Result<Value, ImportReverseDbError> {
    match input {
        ReverseCandidateApplyData::Blueprint(draft) => {
            let result = super::apply_reverse_blueprint_records(txn, project_id, draft).await?;
            Ok(reverse_blueprint_result(result))
        }
        ReverseCandidateApplyData::Outline(rows) => {
            let count = super::apply_reverse_outline_records(txn, project_id, &rows)
                .await
                .map_err(ImportReverseDbError::Database)?;
            Ok(json!({"outline_count": count}))
        }
        ReverseCandidateApplyData::PartOutline(rows) => {
            let count = super::apply_reverse_part_outline_records(txn, project_id, &rows).await?;
            Ok(json!({"part_outline_count": count}))
        }
    }
}

fn reverse_blueprint_result(result: ReverseBlueprintApplyResult) -> Value {
    json!({
        "blueprint_applied": true,
        "blueprint_version": result.version,
        "character_count": result.character_count,
        "relationship_count": result.relationship_count,
    })
}

fn validate_apply_target(input: &ReverseCandidateApplyInput) -> Result<(), ImportReverseDbError> {
    let target_matches = matches!(
        (&input.target[..], &input.apply_data),
        ("reverse_blueprint", ReverseCandidateApplyData::Blueprint(_))
            | ("reverse_outline", ReverseCandidateApplyData::Outline(_))
            | (
                "reverse_part_outline",
                ReverseCandidateApplyData::PartOutline(_)
            )
    );
    if target_matches && (!input.finalize_running_workflow || input.target == "reverse_blueprint") {
        Ok(())
    } else {
        Err(ImportReverseDbError::InvalidArgument(
            "反推 checkpoint 目标与正式写入数据类型不匹配".to_string(),
        ))
    }
}

fn validate_run_snapshot(
    run: &workflow_run::Model,
    input: &ReverseCandidateApplyInput,
) -> Result<(), ImportReverseDbError> {
    let project_id = input.project_id;
    let expected_business_key =
        business_key(project_id, &input.target).map_err(ImportReverseDbError::InvalidArgument)?;
    if run.workflow_kind != WORKFLOW_KIND
        || run.mode.as_deref() != Some(WORKFLOW_MODE)
        || run.project_id != Some(project_id)
        || run.root_project_id != Some(project_id)
        || run.status != input.run.status
        || run.current_step_key.as_deref() != Some(input.target.as_str())
        || run.business_key != expected_business_key
        || run.updated_at.to_string() != input.run.updated_at
        || run.input_fingerprint_json != input.run.input_fingerprint_json
    {
        return Err(ImportReverseDbError::InvalidState {
            current: "reverse_candidate_workflow_changed".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    Ok(())
}

fn validate_checkpoint_snapshot(
    checkpoint: &workflow_checkpoint::Model,
    input: &ReverseCandidateApplyInput,
) -> Result<(), ImportReverseDbError> {
    if checkpoint.id != input.checkpoint.id
        || checkpoint.workflow_run_id != input.workflow_run_id
        || checkpoint.step_key != input.target
        || checkpoint.status != WORKFLOW_STATUS_COMPLETED
        || checkpoint.updated_at.to_string() != input.checkpoint.updated_at
        || checkpoint.input_fingerprint_sha256 != input.checkpoint.input_fingerprint_sha256
        || checkpoint.input_fingerprint_json != input.checkpoint.input_fingerprint_json
        || checkpoint.checkpoint_json != input.checkpoint.checkpoint_json
        || checkpoint.artifacts_json != input.checkpoint.artifacts_json
    {
        return Err(ImportReverseDbError::InvalidState {
            current: "reverse_candidate_checkpoint_changed".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    Ok(())
}

fn validate_candidate_output_fingerprint(
    checkpoint: &workflow_checkpoint::Model,
    output: &Value,
) -> Result<(), ImportReverseDbError> {
    let artifacts = checkpoint
        .artifacts_json
        .as_ref()
        .and_then(|value| value.get("candidate"))
        .and_then(Value::as_object)
        .ok_or_else(|| ImportReverseDbError::InvalidState {
            current: "missing_reverse_candidate".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    let expected = artifacts
        .get("output_fingerprint_sha256")
        .and_then(Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| ImportReverseDbError::InvalidState {
            current: "missing_reverse_candidate_output_fingerprint".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    let actual = crate::generation::generation_checkpoint_fingerprint_sha256(output)
        .map_err(|error| ImportReverseDbError::InvalidArgument(error.to_string()))?;
    if expected == actual {
        Ok(())
    } else {
        Err(ImportReverseDbError::InvalidState {
            current: "reverse_candidate_output_fingerprint_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })
    }
}

fn confirmed_candidate_result(
    candidate: &serde_json::Map<String, Value>,
) -> Result<Option<Value>, ImportReverseDbError> {
    match candidate.get("state").and_then(Value::as_str) {
        Some("proposal") => Ok(None),
        Some("confirmed") => candidate
            .get("apply_result")
            .filter(|value| value.is_object())
            .cloned()
            .map(Some)
            .ok_or_else(|| ImportReverseDbError::InvalidState {
                current: "confirmed_reverse_candidate_missing_result".to_string(),
                operation: "读取已写入的项目资料归纳结果".to_string(),
            }),
        Some(state) => Err(ImportReverseDbError::InvalidState {
            current: format!("invalid_reverse_candidate_state: {state}"),
            operation: "写入项目资料归纳结果".to_string(),
        }),
        None => Err(ImportReverseDbError::InvalidState {
            current: "missing_reverse_candidate_state".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        }),
    }
}

fn applied_status(target: &str) -> Result<ProjectStatus, ImportReverseDbError> {
    match target {
        "reverse_blueprint" => Ok(ProjectStatus::BlueprintReady),
        "reverse_outline" => Ok(ProjectStatus::ChapterOutlinesReady),
        "reverse_part_outline" => Ok(ProjectStatus::PartOutlinesReady),
        _ => Err(ImportReverseDbError::InvalidArgument(format!(
            "未知反推目标: {target}"
        ))),
    }
}

fn project_status_write_error(error: ProjectStatusWriteError) -> ImportReverseDbError {
    match error {
        ProjectStatusWriteError::InvalidArgument(message) => {
            ImportReverseDbError::InvalidArgument(message)
        }
        ProjectStatusWriteError::ProjectNotFound { project_id } => {
            ImportReverseDbError::NotFound(format!("项目不存在: {project_id}"))
        }
        ProjectStatusWriteError::StatusChanged { expected, actual } => {
            ImportReverseDbError::InvalidState {
                current: actual,
                operation: format!("更新项目状态，预期状态 {expected}"),
            }
        }
        ProjectStatusWriteError::Database { source } => ImportReverseDbError::Database(source),
    }
}

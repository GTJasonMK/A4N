//! 导入反推结果的断点恢复与正式资料应用。
//!
//! LLM 结果保存在 workflow checkpoint；应用阶段只从同一 checkpoint 读取并一次事务
//! 写入正式资料，不接受调用方上传的 JSON，也不重新调用模型。蓝图由用户发起归纳后
//! 直接应用，章节与分部大纲继续保留显式确认。

use a4n_api_contract::import_reverse::{
    ReverseEngineeringCandidate, ReverseEngineeringCandidateState,
};
use a4n_db::{
    import_reverse::{
        apply_reverse_blueprint_records, apply_reverse_outline_records,
        apply_reverse_part_outline_records, reverse_context_rows,
    },
    outline::load_outline_generation_source_rows_for_update,
    workflow::completed_llm_call_checkpoint_from_model,
};
use a4n_db_models::models::generation::{workflow_checkpoint, workflow_run};
use a4n_domain::project::ProjectStatus;
use a4n_domain::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING};
use a4n_error::AppError;
use a4n_event::InternalEvent;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::workflow::workflow_checkpoint_contract_app_error;

use super::{
    IMPORT_REVERSE_WORKFLOW_KIND, IMPORT_REVERSE_WORKFLOW_MODE, ReverseEngineeringTarget,
    application::{
        reverse_blueprint_apply_draft, reverse_blueprint_apply_result,
        reverse_outline_apply_result, reverse_outline_rows, reverse_part_outline_apply_result,
        reverse_part_outline_rows, validate_reverse_engineer_output,
    },
    context::ReverseContentRequest,
    execution::restored_import_reverse_checkpoint,
    import_reverse_db_error,
    target::ReverseEngineeringTargetDomain,
    workflow_rules::import_reverse_business_key,
};

pub(crate) const REVERSE_CANDIDATE_ARTIFACT_KEY: &str = "candidate";

/// 在 LLM checkpoint 中保存一个可审阅的候选标记。
pub(crate) fn reverse_candidate_proposal(output_fingerprint_sha256: &str) -> Value {
    json!({
        "state": "proposal",
        "output_fingerprint_sha256": output_fingerprint_sha256,
    })
}

pub(crate) fn reverse_target_from_prompt_name(
    prompt_name: &str,
) -> Result<ReverseEngineeringTarget, AppError> {
    ReverseEngineeringTarget::from_prompt_name(prompt_name).ok_or_else(|| AppError::InvalidState {
        current: prompt_name.to_string(),
        operation: "读取项目资料归纳目标".to_string(),
    })
}

pub(crate) fn reverse_candidate_response(
    workflow_run_id: i64,
    target: ReverseEngineeringTarget,
    state: ReverseEngineeringCandidateState,
    result: Value,
    applied: Option<Value>,
    warnings: Vec<String>,
) -> ReverseEngineeringCandidate {
    ReverseEngineeringCandidate {
        workflow_run_id,
        target,
        state,
        warnings,
        result,
        applied,
    }
}

pub(crate) fn reverse_candidate_checkpoint_response(
    workflow_run_id: i64,
    target: ReverseEngineeringTarget,
    result: Value,
    artifacts: Option<&Value>,
) -> Result<ReverseEngineeringCandidate, AppError> {
    let candidate = artifacts
        .and_then(Value::as_object)
        .and_then(|artifacts| artifacts.get(REVERSE_CANDIDATE_ARTIFACT_KEY))
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate".to_string(),
            operation: "恢复项目资料归纳结果".to_string(),
        })?;
    validate_candidate_output_fingerprint(candidate, &result)?;
    match candidate.get("state").and_then(Value::as_str) {
        Some("proposal") => Ok(reverse_candidate_response(
            workflow_run_id,
            target,
            ReverseEngineeringCandidateState::Proposal,
            result,
            None,
            Vec::new(),
        )),
        Some("confirmed") => Ok(reverse_candidate_response(
            workflow_run_id,
            target,
            ReverseEngineeringCandidateState::Confirmed,
            result,
            confirmed_reverse_candidate_apply_result(candidate)?,
            Vec::new(),
        )),
        Some(state) => Err(AppError::InvalidState {
            current: state.to_string(),
            operation: "恢复项目资料归纳结果".to_string(),
        }),
        None => Err(AppError::InvalidState {
            current: "missing_reverse_candidate_state".to_string(),
            operation: "恢复项目资料归纳结果".to_string(),
        }),
    }
}

impl OrchestratorContext {
    pub async fn confirm_reverse_engineering_candidate_validated(
        &self,
        workflow_run_id: i64,
    ) -> Result<ReverseEngineeringCandidate, AppError> {
        confirm_reverse_engineering_candidate(self, workflow_run_id).await
    }

    pub async fn latest_reverse_engineering_candidate_validated(
        &self,
        project_id: i64,
        target: ReverseEngineeringTarget,
    ) -> Result<Option<ReverseEngineeringCandidate>, AppError> {
        latest_reverse_engineering_candidate(self, project_id, target).await
    }
}

async fn latest_reverse_engineering_candidate(
    context: &OrchestratorContext,
    project_id: i64,
    target: ReverseEngineeringTarget,
) -> Result<Option<ReverseEngineeringCandidate>, AppError> {
    if project_id <= 0 {
        return Err(AppError::InvalidArgument(
            "project_id 必须大于 0".to_string(),
        ));
    }
    let prompt_name = target.prompt_name();
    let business_key =
        import_reverse_business_key(project_id, target).map_err(AppError::InvalidArgument)?;
    let Some(run) = workflow_run::Entity::find()
        .filter(workflow_run::Column::BusinessKey.eq(business_key))
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::Status.eq(WORKFLOW_STATUS_COMPLETED))
        .order_by_desc(workflow_run::Column::UpdatedAt)
        .order_by_desc(workflow_run::Column::Id)
        .one(context.database.connection())
        .await?
    else {
        return Ok(None);
    };
    validate_reverse_candidate_run(&run, project_id, target, WORKFLOW_STATUS_COMPLETED)?;
    let checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(run.id))
        .filter(workflow_checkpoint::Column::StepKey.eq(prompt_name))
        .one(context.database.connection())
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "工作流 {} 缺少反推 checkpoint {prompt_name}",
                run.id
            ))
        })?;
    validate_reverse_candidate_checkpoint(&checkpoint, run.id, target)?;
    let checkpoint = completed_llm_call_checkpoint_from_model(&checkpoint, prompt_name)
        .map_err(workflow_checkpoint_contract_app_error)?;
    let result = restored_import_reverse_checkpoint(
        &checkpoint.parsed_output,
        &checkpoint.response_content,
        target,
    )?;
    reverse_candidate_checkpoint_response(
        run.id,
        target,
        result,
        checkpoint.artifacts_json.as_ref(),
    )
    .map(Some)
}

pub(crate) async fn confirm_reverse_engineering_candidate(
    context: &OrchestratorContext,
    workflow_run_id: i64,
) -> Result<ReverseEngineeringCandidate, AppError> {
    apply_reverse_engineering_checkpoint(context, workflow_run_id, None, WORKFLOW_STATUS_COMPLETED)
        .await
}

pub(crate) async fn apply_reverse_blueprint_workflow_result(
    context: &OrchestratorContext,
    workflow_run_id: i64,
) -> Result<ReverseEngineeringCandidate, AppError> {
    apply_reverse_engineering_checkpoint(
        context,
        workflow_run_id,
        Some(ReverseEngineeringTarget::Blueprint),
        WORKFLOW_STATUS_RUNNING,
    )
    .await
}

async fn apply_reverse_engineering_checkpoint(
    context: &OrchestratorContext,
    workflow_run_id: i64,
    expected_target: Option<ReverseEngineeringTarget>,
    expected_run_status: &str,
) -> Result<ReverseEngineeringCandidate, AppError> {
    if workflow_run_id <= 0 {
        return Err(AppError::InvalidArgument(
            "workflow_run_id 必须大于 0".to_string(),
        ));
    }

    let tx = context.database.connection().begin().await?;

    // 自动上下文依赖项目正式资料；项目行先串行化正式写入，再锁正文与 workflow run。
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
                operation: "写入项目资料归纳结果".to_string(),
            })?;
    let locked_sources = load_outline_generation_source_rows_for_update(&tx, project_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let project = locked_sources.project;
    if project.lifecycle_gate != a4n_db::generation::PROJECT_GATE_ACTIVE {
        return Err(AppError::InvalidState {
            current: project.lifecycle_gate,
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
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
            current: format!(
                "workflow_run_id={}, status={}, business_key={}",
                run.id, run.status, run.business_key
            ),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    let target = reverse_target_from_prompt_name(prompt_name)?;
    if expected_target.is_some_and(|expected| expected != target) {
        return Err(AppError::InvalidState {
            current: prompt_name.to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    validate_reverse_candidate_run(&run, project_id, target, expected_run_status)?;
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
    validate_reverse_candidate_checkpoint(&checkpoint, workflow_run_id, target)?;

    let completed_checkpoint = completed_llm_call_checkpoint_from_model(&checkpoint, prompt_name)
        .map_err(workflow_checkpoint_contract_app_error)?;
    let parsed_output = restored_import_reverse_checkpoint(
        &completed_checkpoint.parsed_output,
        &completed_checkpoint.response_content,
        target,
    )?;
    validate_reverse_engineer_output(target, &parsed_output)?;

    let mut artifacts =
        completed_checkpoint
            .artifacts_json
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_reverse_candidate_artifacts".to_string(),
                operation: "写入项目资料归纳结果".to_string(),
            })?;
    let artifacts_object = artifacts
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate_artifacts".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    let mut candidate = artifacts_object
        .get(REVERSE_CANDIDATE_ARTIFACT_KEY)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    let candidate_object = candidate
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    validate_candidate_output_fingerprint(candidate_object, &parsed_output)?;

    if let Some(applied) = confirmed_reverse_candidate_apply_result(candidate_object)? {
        tx.rollback().await?;
        let current_status =
            a4n_db::project::project_by_id_record(context.database.connection(), project_id)
                .await
                .map_err(crate::project::project_mutation_error_to_app_error)?
                .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?
                .status;
        let current_status = ProjectStatus::parse_storage(&current_status)?;
        if current_status < target.applied_status() {
            return Err(AppError::InvalidState {
                current: current_status.as_str().to_string(),
                operation: format!(
                    "读取已确认的项目资料归纳结果（期望项目状态至少为 {}）",
                    target.applied_status().as_str()
                ),
            });
        }
        publish_reverse_blueprint_updated(context, project_id, target, &applied)?;
        return Ok(reverse_candidate_response(
            workflow_run_id,
            target,
            ReverseEngineeringCandidateState::Confirmed,
            parsed_output,
            Some(applied),
            Vec::new(),
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
                operation: "写入项目资料归纳结果".to_string(),
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
                .map_err(AppError::Database)?;
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
    let current_status = ProjectStatus::parse_storage(&project.status)?;
    let target_status = target.applied_status();
    let status_change = if current_status < target_status {
        a4n_db::project::update_project_status_record_in(&tx, project, target_status, now)
            .await
            .map_err(crate::project::project_status_write_error_to_app_error)?;
        Some((current_status, target_status))
    } else {
        None
    };
    candidate_object.insert("state".to_string(), json!("confirmed"));
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

    if let Some((from, to)) = status_change {
        context.publish_internal_event(InternalEvent::ProjectStatusChanged {
            project_id,
            from,
            to,
            timestamp: now,
        })?;
    }
    publish_reverse_blueprint_updated(context, project_id, target, &applied)?;
    Ok(reverse_candidate_response(
        workflow_run_id,
        target,
        ReverseEngineeringCandidateState::Confirmed,
        parsed_output,
        Some(applied),
        Vec::new(),
    ))
}

fn publish_reverse_blueprint_updated(
    context: &OrchestratorContext,
    project_id: i64,
    target: ReverseEngineeringTarget,
    applied: &Value,
) -> Result<(), AppError> {
    if target != ReverseEngineeringTarget::Blueprint {
        return Ok(());
    }
    let version = applied
        .get("blueprint_version")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_applied_blueprint_version".to_string(),
            operation: "发布蓝图归纳结果".to_string(),
        })?;
    context.publish_internal_event(a4n_event::InternalEvent::BlueprintUpdated {
        project_id,
        version: version.to_string(),
    })?;
    Ok(())
}

fn validate_reverse_candidate_run(
    run: &workflow_run::Model,
    project_id: i64,
    target: ReverseEngineeringTarget,
    expected_status: &str,
) -> Result<(), AppError> {
    let prompt_name = target.prompt_name();
    if run.workflow_kind != IMPORT_REVERSE_WORKFLOW_KIND {
        return Err(AppError::InvalidArgument(
            "workflow_run_id 不是有效的项目资料归纳工作流".to_string(),
        ));
    }
    if run.mode.as_deref() != Some(IMPORT_REVERSE_WORKFLOW_MODE) {
        return Err(AppError::InvalidArgument(
            "只能写入 reverse 模式的项目资料归纳结果".to_string(),
        ));
    }
    if run.project_id != Some(project_id) || run.root_project_id != Some(project_id) {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_project_scope_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    if run.status != expected_status {
        return Err(AppError::InvalidState {
            current: run.status.clone(),
            operation: format!("写入项目资料归纳结果（工作流必须为 {expected_status}）"),
        });
    }
    if run.current_step_key.as_deref() != Some(prompt_name) {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_step_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    let fingerprint =
        run.input_fingerprint_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_reverse_candidate_input_fingerprint".to_string(),
                operation: "写入项目资料归纳结果".to_string(),
            })?;
    validate_reverse_candidate_run_fingerprint(fingerprint, project_id, target)?;
    let expected_business_key =
        import_reverse_business_key(project_id, target).map_err(AppError::InvalidArgument)?;
    if run.business_key != expected_business_key {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_business_key_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    Ok(())
}

fn validate_reverse_candidate_run_fingerprint(
    fingerprint: &Value,
    project_id: i64,
    target: ReverseEngineeringTarget,
) -> Result<(), AppError> {
    let prompt_name = target.prompt_name();
    let expected = serde_json::json!({
        "schema_version": 1,
        "workflow": IMPORT_REVERSE_WORKFLOW_KIND,
        "target": prompt_name,
        "mode": IMPORT_REVERSE_WORKFLOW_MODE,
        "project_id": project_id,
    });
    for key in ["schema_version", "workflow", "target", "mode", "project_id"] {
        if fingerprint.get(key) != expected.get(key) {
            return Err(AppError::InvalidState {
                current: format!("reverse_candidate_input_fingerprint_{key}_mismatch"),
                operation: "写入项目资料归纳结果".to_string(),
            });
        }
    }
    fingerprint
        .get("content_sample_sha256")
        .and_then(Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate_content_sample_fingerprint".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
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
            operation: "写入项目资料归纳结果".to_string(),
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
            operation: "写入项目资料归纳结果".to_string(),
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
    let auto_context = fingerprint
        .get("auto_context")
        .and_then(Value::as_bool)
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate_input_fingerprint_shape".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    if !auto_context {
        return Ok(());
    }

    let expected_hash = fingerprint
        .get("auto_context_source_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate_auto_context_fingerprint".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    let context_limit = fingerprint
        .get("context_limit")
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_reverse_candidate_context_limit".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
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
        operation: "写入项目资料归纳结果".to_string(),
    })?;
    validate_auto_context_source_hash(expected_hash, &current_hash)
}

fn validate_auto_context_source_hash(expected: &str, current: &str) -> Result<(), AppError> {
    if expected == current {
        return Ok(());
    }
    Err(AppError::InvalidState {
        current: "reverse_candidate_source_changed".to_string(),
        operation: "写入项目资料归纳结果（项目上下文已变化，请重新归纳）".to_string(),
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
    target: ReverseEngineeringTarget,
) -> Result<(), AppError> {
    let prompt_name = target.prompt_name();
    if checkpoint.workflow_run_id != workflow_run_id
        || checkpoint.step_key != prompt_name
        || checkpoint.step_order != 1
    {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_checkpoint_identity_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    if checkpoint.status != WORKFLOW_STATUS_COMPLETED {
        return Err(AppError::InvalidState {
            current: checkpoint.status.clone(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    let fingerprint =
        checkpoint
            .input_fingerprint_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_reverse_candidate_checkpoint_fingerprint".to_string(),
                operation: "写入项目资料归纳结果".to_string(),
            })?;
    let expected = serde_json::json!({
        "schema_version": 1,
        "workflow": IMPORT_REVERSE_WORKFLOW_KIND,
        "call_key": prompt_name,
        "prompt_name": prompt_name,
    });
    for key in ["schema_version", "workflow", "call_key", "prompt_name"] {
        if fingerprint.get(key) != expected.get(key) {
            return Err(AppError::InvalidState {
                current: format!("reverse_candidate_checkpoint_{key}_mismatch"),
                operation: "写入项目资料归纳结果".to_string(),
            });
        }
    }
    let expected_hash = a4n_db::workflow::llm_call_checkpoint_fingerprint_sha256(fingerprint)
        .map_err(|error| AppError::InvalidState {
            current: format!("invalid_reverse_candidate_checkpoint_fingerprint: {error}"),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    if checkpoint.input_fingerprint_sha256.as_deref() != Some(expected_hash.as_str()) {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_checkpoint_fingerprint_hash_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        });
    }
    Ok(())
}

fn validate_candidate_output_fingerprint(
    candidate: &serde_json::Map<String, Value>,
    parsed_output: &Value,
) -> Result<(), AppError> {
    let output_fingerprint = candidate
        .get("output_fingerprint_sha256")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_reverse_candidate_output_fingerprint".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        })?;
    let actual = a4n_db::generation::generation_checkpoint_fingerprint_sha256(parsed_output)
        .map_err(AppError::Serialization)?;
    if output_fingerprint != actual {
        return Err(AppError::InvalidState {
            current: "reverse_candidate_output_fingerprint_mismatch".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
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
                operation: "读取已写入的项目资料归纳结果".to_string(),
            }),
        Some(other) => Err(AppError::InvalidState {
            current: other.to_string(),
            operation: "写入项目资料归纳结果".to_string(),
        }),
        None => Err(AppError::InvalidState {
            current: "missing_reverse_candidate_state".to_string(),
            operation: "写入项目资料归纳结果".to_string(),
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
        let target = ReverseEngineeringTarget::Outline;
        let prompt_name = target.prompt_name();
        workflow_run::Model {
            id: 42,
            business_key: import_reverse_business_key(7, target).unwrap(),
            workflow_kind: IMPORT_REVERSE_WORKFLOW_KIND.to_string(),
            project_id: Some(7),
            root_project_id: Some(7),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(7),
            chapter_id: None,
            mode: Some(IMPORT_REVERSE_WORKFLOW_MODE.to_string()),
            input_fingerprint_json: Some(json!({
                "schema_version": 1,
                "workflow": IMPORT_REVERSE_WORKFLOW_KIND,
                "target": prompt_name,
                "mode": IMPORT_REVERSE_WORKFLOW_MODE,
                "project_id": 7,
                "content_sample_sha256": "a".repeat(64),
                "auto_context": true,
                "context_limit": 8,
                "auto_context_source_sha256": "b".repeat(64),
                "variables": {},
            })),
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            current_step_key: Some(prompt_name.to_string()),
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
        let prompt_name = ReverseEngineeringTarget::Outline.prompt_name();
        let fingerprint = json!({
            "schema_version": 1,
            "workflow": IMPORT_REVERSE_WORKFLOW_KIND,
            "call_key": prompt_name,
            "prompt_name": prompt_name,
            "messages": [],
            "context_manifest": null,
            "max_tokens": 4096,
            "response_format": null,
        });
        let hash = a4n_db::workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint).unwrap();
        workflow_checkpoint::Model {
            id: 9,
            workflow_run_id: 42,
            step_key: prompt_name.to_string(),
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
    fn candidate_proposal_only_persists_mutable_candidate_state() {
        let proposal = reverse_candidate_proposal("output-hash");
        assert_eq!(proposal["state"], "proposal");
        assert_eq!(proposal["output_fingerprint_sha256"], "output-hash");
        assert_eq!(proposal.as_object().unwrap().len(), 2);
        assert_eq!(
            reverse_target_from_prompt_name("reverse_part_outline").unwrap(),
            ReverseEngineeringTarget::PartOutline
        );
    }

    #[test]
    fn checkpoint_response_keeps_candidate_separate_from_formal_apply() {
        let output = json!({"outlines": []});
        let output_hash =
            a4n_db::generation::generation_checkpoint_fingerprint_sha256(&output).unwrap();
        let artifacts = json!({
            (REVERSE_CANDIDATE_ARTIFACT_KEY):
                reverse_candidate_proposal(&output_hash),
        });

        let response = reverse_candidate_checkpoint_response(
            42,
            ReverseEngineeringTarget::Outline,
            output.clone(),
            Some(&artifacts),
        )
        .unwrap();

        assert_eq!(response.state, ReverseEngineeringCandidateState::Proposal);
        assert_eq!(response.result, output);
        assert!(response.applied.is_none());
    }

    #[test]
    fn unknown_candidate_target_is_rejected() {
        assert!(reverse_target_from_prompt_name("reverse_unknown").is_err());
    }

    #[test]
    fn unfinished_workflow_cannot_be_confirmed() {
        let mut run = completed_run();
        run.status = "running".to_string();

        let error = validate_reverse_candidate_run(
            &run,
            7,
            ReverseEngineeringTarget::Outline,
            WORKFLOW_STATUS_COMPLETED,
        )
        .unwrap_err();

        assert_eq!(invalid_state(error), "running");
    }

    #[test]
    fn workflow_fingerprint_tampering_is_rejected() {
        let mut run = completed_run();
        run.input_fingerprint_json.as_mut().unwrap()["workflow"] = json!("other_workflow");

        let error = validate_reverse_candidate_run(
            &run,
            7,
            ReverseEngineeringTarget::Outline,
            WORKFLOW_STATUS_COMPLETED,
        )
        .unwrap_err();

        assert_eq!(
            invalid_state(error),
            "reverse_candidate_input_fingerprint_workflow_mismatch"
        );
    }

    #[test]
    fn malformed_workflow_source_fingerprint_is_rejected() {
        let mut run = completed_run();
        run.input_fingerprint_json.as_mut().unwrap()["content_sample_sha256"] = json!("short");

        let error = validate_reverse_candidate_run(
            &run,
            7,
            ReverseEngineeringTarget::Outline,
            WORKFLOW_STATUS_COMPLETED,
        )
        .unwrap_err();

        assert_eq!(
            invalid_state(error),
            "invalid_reverse_candidate_content_sample_fingerprint"
        );
    }

    #[test]
    fn checkpoint_fingerprint_tampering_is_rejected() {
        let mut checkpoint = completed_checkpoint();
        checkpoint.input_fingerprint_json.as_mut().unwrap()["messages"] = json!([{"role": "user"}]);

        let error = validate_reverse_candidate_checkpoint(
            &checkpoint,
            42,
            ReverseEngineeringTarget::Outline,
        )
        .unwrap_err();

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

        let error = validate_reverse_candidate_checkpoint(
            &checkpoint,
            42,
            ReverseEngineeringTarget::Outline,
        )
        .unwrap_err();

        assert_eq!(
            invalid_state(error),
            "reverse_candidate_checkpoint_call_key_mismatch"
        );
    }

    #[test]
    fn candidate_output_tampering_is_rejected() {
        let output = json!({"outlines": []});
        let output_hash =
            a4n_db::generation::generation_checkpoint_fingerprint_sha256(&output).unwrap();
        let candidate = reverse_candidate_proposal(&output_hash);
        validate_candidate_output_fingerprint(candidate.as_object().unwrap(), &output).unwrap();

        let mut tampered = candidate;
        tampered["output_fingerprint_sha256"] = json!("tampered");
        let error = validate_candidate_output_fingerprint(tampered.as_object().unwrap(), &output)
            .unwrap_err();
        assert_eq!(
            invalid_state(error),
            "reverse_candidate_output_fingerprint_mismatch"
        );
    }

    #[test]
    fn confirmed_candidate_returns_stored_result_without_reapplying() {
        let mut candidate = reverse_candidate_proposal("output-hash");
        candidate["state"] = json!("confirmed");
        candidate["apply_result"] = json!({"outline_count": 3});

        let applied = confirmed_reverse_candidate_apply_result(candidate.as_object().unwrap())
            .unwrap()
            .expect("confirmed candidate must take the idempotent return path");

        assert_eq!(applied, json!({"outline_count": 3}));
        assert!(
            confirmed_reverse_candidate_apply_result(
                reverse_candidate_proposal("output-hash")
                    .as_object()
                    .unwrap()
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn confirmed_candidate_requires_structured_apply_result() {
        let mut candidate = reverse_candidate_proposal("output-hash");
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
        validate_reverse_candidate_run(
            &run,
            7,
            ReverseEngineeringTarget::Outline,
            WORKFLOW_STATUS_COMPLETED,
        )
        .expect("manual samples use immutable checkpoint text only");
    }
}

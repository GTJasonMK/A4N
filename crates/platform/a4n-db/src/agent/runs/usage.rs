use super::*;

pub async fn update_run_usage(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    usage_json: Value,
) -> Result<run::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&row, lease_token, timestamp)?;
    // The process-local snapshot requirement is an admission invariant. Worker
    // writes must not erase it with stale usage data and permit an unsafe resume.
    let mut usage_json = usage_json;
    let snapshot_required = match row.usage_json.get("runtime_snapshot_required") {
        Some(Value::Bool(required)) => *required,
        Some(_) => {
            return Err(AgentDbError::InvalidArgument(format!(
                "Agent run {run_id} runtime_snapshot_required 必须是 boolean"
            )));
        }
        None => {
            return Err(AgentDbError::InvalidArgument(format!(
                "Agent run {run_id} 缺少 runtime_snapshot_required 标记"
            )));
        }
    };
    let usage_object = usage_json
        .as_object_mut()
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent run usage_json 必须是对象".into()))?;
    for field in [
        "steps_used",
        "model_calls_used",
        "tool_calls_used",
        "tokens_used",
        "tokens_reserved",
        "usage_complete",
        "budget_limiting_run_id",
    ] {
        usage_object.remove(field);
    }
    usage_object.insert(
        "runtime_snapshot_required".into(),
        Value::Bool(snapshot_required),
    );
    // Control-plane wait/lineage markers cannot be erased by stale Worker
    // progress. They are not counters and are cleared by their lifecycle owner.
    for field in ["budget_wait", "paused_by_run_id"] {
        usage_object.remove(field);
        if let Some(value) = row.usage_json.get(field) {
            usage_object.insert(field.into(), value.clone());
        }
    }
    let mut active: run::ActiveModel = row.into();
    active.usage_json = Set(usage_json);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row)
}

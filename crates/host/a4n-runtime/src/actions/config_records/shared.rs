pub(super) fn validate_runtime_snapshot_config_id(
    config_kind: &str,
    snapshot_id: i64,
    config_id: i64,
) -> Result<(), String> {
    if snapshot_id == config_id {
        return Ok(());
    }
    Err(format!(
        "runtime_config_snapshot.{config_kind}.id 必须等于 config_id：snapshot_id={snapshot_id}, config_id={config_id}",
    ))
}

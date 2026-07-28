use crate::AppState;
use crate::actions::image_records;
pub use a4n_core::dependency_conflict::DownstreamConflictStrategy;

pub(crate) fn parse_downstream_conflict_strategy(
    value: &str,
) -> Result<DownstreamConflictStrategy, String> {
    if value.trim().is_empty() {
        return Err(
            "下游冲突处理策略不能为空，支持 reject、keep_downstream、delete_downstream".to_string(),
        );
    }
    DownstreamConflictStrategy::from_optional_str(Some(value))
}

pub(crate) fn resolve_downstream_id_conflict(
    ids: Vec<i64>,
    strategy: DownstreamConflictStrategy,
    subject: &str,
    downstream_label: &str,
    operation: &str,
    source_label: &str,
) -> Result<Vec<i64>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    match strategy {
        DownstreamConflictStrategy::Reject => Err(format!(
            "{subject}已派生 {} {downstream_label}，{operation}前需要选择：删除下游派生产物，或保留下游只覆盖{source_label}",
            ids.len()
        )),
        DownstreamConflictStrategy::KeepDownstream => Ok(Vec::new()),
        DownstreamConflictStrategy::DeleteDownstream => Ok(ids),
    }
}

pub(crate) async fn delete_downstream_images_if_requested(
    state: &AppState,
    user_id: i64,
    image_ids: impl IntoIterator<Item = i64>,
) -> Result<(), String> {
    for image_id in image_ids {
        image_records::delete_image(state, user_id, image_id).await?;
    }
    Ok(())
}

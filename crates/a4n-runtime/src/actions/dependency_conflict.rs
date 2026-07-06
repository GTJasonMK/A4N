use crate::AppState;
use crate::actions::image_records;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownstreamConflictStrategy {
    #[default]
    Reject,
    KeepDownstream,
    DeleteDownstream,
}

impl DownstreamConflictStrategy {
    pub fn from_optional_str(value: Option<&str>) -> Result<Self, String> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None => Ok(Self::Reject),
            Some("reject") => Ok(Self::Reject),
            Some("keep_downstream") => Ok(Self::KeepDownstream),
            Some("delete_downstream") => Ok(Self::DeleteDownstream),
            Some(other) => Err(format!(
                "未知下游冲突处理策略：{other}，支持 reject、keep_downstream、delete_downstream"
            )),
        }
    }
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

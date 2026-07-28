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
            None | Some("reject") => Ok(Self::Reject),
            Some("keep_downstream") => Ok(Self::KeepDownstream),
            Some("delete_downstream") => Ok(Self::DeleteDownstream),
            Some(other) => Err(format!(
                "未知下游冲突处理策略：{other}，支持 reject、keep_downstream、delete_downstream"
            )),
        }
    }
}

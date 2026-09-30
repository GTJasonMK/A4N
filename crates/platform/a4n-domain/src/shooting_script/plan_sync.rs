//! 拍摄剧本与已确认导演编排的逐镜头同步。
//!
//! 修改导演编排的某个镜头只应让该镜头过时：
//! 确认新方案时拍摄剧本被变基，内容未变的镜头原样保留，内容变化的镜头保留旧约束作为
//! 过时标记，随后通过手动确认（重绑定）或模型辅助更新指定镜头。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod fingerprints;
mod rebase;
#[cfg(test)]
mod tests;
mod updates;

/// 拍摄剧本相对当前已确认导演编排的逐镜头同步状态。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingScriptPlanSyncV1 {
    /// 导演编排内容已变化的镜头；剧本保留旧内容，等待手动确认或模型辅助更新。
    pub stale_shot_ids: Vec<String>,
    /// 导演编排中存在、剧本尚未覆盖的镜头；需要补全对应拍摄设计。
    pub missing_shot_ids: Vec<String>,
}

impl ShootingScriptPlanSyncV1 {
    pub fn is_current(&self) -> bool {
        self.stale_shot_ids.is_empty() && self.missing_shot_ids.is_empty()
    }
}

//! 后台任务进度展示元数据。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgressPhaseKind {
    Prepare,
    Context,
    ModelConnect,
    ModelWait,
    ModelStream,
    StructuredOutput,
    Parse,
    Validate,
    Save,
    AssetGeneration,
    Transfer,
    Queue,
    Cleanup,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProgressSeverity {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProgressMeta {
    pub task_kind: String,
    pub stage_code: String,
    pub stage_label: String,
    pub phase_kind: ProgressPhaseKind,
    pub detail: Option<String>,
    pub unit_label: Option<String>,
    pub item_index: Option<u64>,
    pub item_total: Option<u64>,
    pub parent_stage_code: Option<String>,
    pub severity: ProgressSeverity,
    pub estimated: bool,
    pub persistent: bool,
}

impl ProgressMeta {
    pub fn new(
        task_kind: impl Into<String>,
        stage_code: impl Into<String>,
        stage_label: impl Into<String>,
        phase_kind: ProgressPhaseKind,
    ) -> Self {
        Self {
            task_kind: task_kind.into(),
            stage_code: stage_code.into(),
            stage_label: stage_label.into(),
            phase_kind,
            detail: None,
            unit_label: None,
            item_index: None,
            item_total: None,
            parent_stage_code: None,
            severity: ProgressSeverity::Info,
            estimated: false,
            persistent: false,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_unit(
        mut self,
        unit_label: impl Into<String>,
        item_index: Option<u64>,
        item_total: Option<u64>,
    ) -> Self {
        self.unit_label = Some(unit_label.into());
        self.item_index = item_index;
        self.item_total = item_total;
        self
    }
}

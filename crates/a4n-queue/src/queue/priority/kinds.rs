use a4n_core::error::AppError;
pub use a4n_core::event::channel::TaskKind;
use serde::{Deserialize, Serialize};

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    /// 用户交互：灵感对话、章节生成
    High,
    /// 正常工作流：大纲生成、蓝图生成
    Medium,
    /// 后台任务：RAG 入库、导入分析
    Low,
}

impl TaskPriority {
    const ALL: [Self; 3] = [Self::High, Self::Medium, Self::Low];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|priority| priority.as_str() == value)
    }

    pub fn parse_storage(value: &str) -> Result<Self, AppError> {
        Self::parse(value)
            .ok_or_else(|| AppError::InvalidArgument(format!("未知任务优先级: {value}")))
    }
}

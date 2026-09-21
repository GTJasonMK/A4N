//! 章节正文与结构化拍摄剧本之间的导演编排合同。
//!
//! 该合同只表达用户确认过的叙事、时间和画面约束；视频、漫画与 Galgame
//! 可以从同一份方案投影，不能各自重新解释章节。

mod contracts;
mod contracts_v3;
mod editing_v3;
mod validation_v3;

pub use contracts::*;
pub use contracts_v3::*;

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
#[error("章节视觉编排字段 {path} 无效: {message}")]
pub struct ChapterVisualPlanValidationError {
    pub path: String,
    pub message: String,
}

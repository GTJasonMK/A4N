//! 章节结构化拍摄剧本合同。
//!
//! 视频、漫画和 Galgame 只能从该合同投影媒介执行计划，不得重新解释章节并维护
//! 第二套场景、镜头、表演节拍或连续性事实。

mod contracts;
mod validation;

pub use contracts::*;
pub use validation::ShootingScriptValidationError;

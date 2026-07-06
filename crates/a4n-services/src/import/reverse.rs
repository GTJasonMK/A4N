//! 导入反推上下文和写回逻辑。
//!
//! 命令层只负责 API、事务和状态推进；这里负责从 canonical 数据构建反推输入，
//! 并把反推结果写回 canonical 表。

mod apply;
mod context;
mod types;

pub use apply::{
    apply_reverse_blueprint_data, apply_reverse_outline_data, apply_reverse_part_outline_data,
};
pub use context::resolve_reverse_content_sample;
pub use types::{ReverseContentRequest, ReverseContextKind};

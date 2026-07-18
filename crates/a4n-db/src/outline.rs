//! 大纲分部/章节大纲的 DB 边界 helper。

mod chapters;
mod parts;
mod ranges;
mod types;

pub use chapters::*;
pub use parts::*;
pub use ranges::{chapter_outline_ids_in_part_range, chapter_outline_ids_in_part_ranges};
pub use types::*;

#[cfg(test)]
mod tests;

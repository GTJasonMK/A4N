use a4n_core::models::chapter::{chapter_outline, part_outline};
use serde_json::Value;

pub(in crate::outline) struct ChapterOutlineGenerationContext {
    pub(in crate::outline) blueprint_payload: Value,
    pub(in crate::outline) parts: Vec<part_outline::Model>,
    pub(in crate::outline) chapter_outlines: Vec<chapter_outline::Model>,
    pub(in crate::outline) source_snapshot: Value,
}

pub(in crate::outline) struct PartOutlineGenerationContext {
    pub(in crate::outline) chapters_per_part: i32,
    pub(in crate::outline) requested_part_count: i32,
    pub(in crate::outline) blueprint_payload: Value,
    pub(in crate::outline) parts: Vec<part_outline::Model>,
    pub(in crate::outline) source_snapshot: Value,
}

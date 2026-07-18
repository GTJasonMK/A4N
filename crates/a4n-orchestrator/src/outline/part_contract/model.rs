use serde_json::Value;

pub(in crate::outline) struct PartOutlineDraft {
    pub(in crate::outline) part_number: i32,
    pub(in crate::outline) start_chapter: i32,
    pub(in crate::outline) end_chapter: i32,
    pub(in crate::outline) data: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::outline) struct PartGenerationPlan {
    pub(in crate::outline) chapters_per_part: i32,
    pub(in crate::outline) requested_part_count: i32,
}

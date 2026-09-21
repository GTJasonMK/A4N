#[derive(Debug, Clone, Copy)]
pub struct ProgressSpan {
    start: u32,
    end: u32,
}

impl ProgressSpan {
    pub const fn new(start: u32, end: u32) -> Self {
        assert!(start <= end, "progress span start must not exceed end");
        Self { start, end }
    }

    pub const fn start(self) -> u32 {
        self.start
    }

    pub const fn end(self) -> u32 {
        self.end
    }

    pub fn split(self, index: usize, total: usize) -> Self {
        let total = total.max(1);
        let index = index.min(total - 1);
        let width = self.end - self.start;
        let start = self.start + scaled_progress_offset(width, index as u128, total as u128);
        let end = self.start + scaled_progress_offset(width, (index + 1) as u128, total as u128);
        Self { start, end }
    }

    pub(super) fn current_for_units(self, completed_units: u128, total_units: u128) -> u32 {
        let total_units = total_units.max(1);
        let width = self.end - self.start;
        self.start + scaled_progress_offset(width, completed_units.min(total_units), total_units)
    }
}

fn scaled_progress_offset(width: u32, completed: u128, total: u128) -> u32 {
    let offset = u128::from(width) * completed / total;
    u32::try_from(offset).expect("progress offset is bounded by the u32 span width")
}

#[derive(Debug, Clone, Copy)]
pub struct FieldMilestone {
    pub(super) key: &'static str,
    pub(super) label: &'static str,
    pub(super) weight: u32,
}

impl FieldMilestone {
    pub const fn new(key: &'static str, label: &'static str, weight: u32) -> Self {
        Self { key, label, weight }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StructuredJsonProgressSpec {
    pub root_array_key: &'static str,
    pub item_label: &'static str,
    pub expected_items: Option<usize>,
    pub milestones: &'static [FieldMilestone],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredProgressEvent {
    pub current: u32,
    pub total: u32,
    pub message: String,
}

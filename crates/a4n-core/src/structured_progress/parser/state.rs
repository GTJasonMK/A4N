use std::collections::HashSet;

use crate::structured_progress::{ProgressSpan, StructuredJsonProgressSpec};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContainerKind {
    Object,
    Array,
}

#[derive(Debug, Clone)]
pub(super) struct Container {
    pub(super) kind: ContainerKind,
    pub(super) expecting_key: bool,
    pub(super) pending_key: Option<String>,
    pub(super) is_root_array: bool,
    pub(super) is_root_item: bool,
    pub(super) field_key_for_root_item: Option<String>,
}

impl Container {
    pub(super) fn object(is_root_item: bool, field_key_for_root_item: Option<String>) -> Self {
        Self {
            kind: ContainerKind::Object,
            expecting_key: true,
            pending_key: None,
            is_root_array: false,
            is_root_item,
            field_key_for_root_item,
        }
    }

    pub(super) fn array(is_root_array: bool, field_key_for_root_item: Option<String>) -> Self {
        Self {
            kind: ContainerKind::Array,
            expecting_key: false,
            pending_key: None,
            is_root_array,
            is_root_item: false,
            field_key_for_root_item,
        }
    }
}

#[derive(Default)]
pub(in crate::structured_progress) struct ItemProgress {
    pub(in crate::structured_progress) index: usize,
    pub(in crate::structured_progress) completed_fields: HashSet<&'static str>,
    pub(in crate::structured_progress) completed_units: u128,
    pub(in crate::structured_progress) completed: bool,
}

pub struct StructuredJsonProgressTracker {
    pub(in crate::structured_progress) spec: StructuredJsonProgressSpec,
    pub(in crate::structured_progress) span: ProgressSpan,
    pub(super) started_json: bool,
    pub(super) in_string: bool,
    pub(super) escaping: bool,
    pub(super) string_buffer: String,
    pub(super) stack: Vec<Container>,
    pub(in crate::structured_progress) current_item: Option<ItemProgress>,
    pub(in crate::structured_progress) completed_units: u128,
    pub(super) item_count: usize,
    pub(in crate::structured_progress) unit_per_item: u128,
    pub(in crate::structured_progress) max_reported_current: u32,
}

impl StructuredJsonProgressTracker {
    pub fn new(spec: StructuredJsonProgressSpec, span: ProgressSpan) -> Self {
        let milestone_units = spec.milestones.iter().fold(0_u128, |total, milestone| {
            total
                .checked_add(u128::from(milestone.weight.max(1)))
                .expect("structured progress milestone weights exceed u128")
        });
        let unit_per_item = milestone_units
            .checked_add(1)
            .expect("structured progress units per item exceed u128");
        Self {
            spec,
            span,
            started_json: false,
            in_string: false,
            escaping: false,
            string_buffer: String::new(),
            stack: Vec::new(),
            current_item: None,
            completed_units: 0,
            item_count: 0,
            unit_per_item,
            max_reported_current: span.start(),
        }
    }
}

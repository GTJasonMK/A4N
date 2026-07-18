use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::engine::PromptEntry;
use crate::registry::PromptRegistry;

pub(super) struct PlannedPrompt {
    pub(super) entry: PromptEntry,
    pub(super) content: String,
}

pub(super) struct PromptLoadPlan {
    pub(super) prompts: Vec<PlannedPrompt>,
    pub(super) skipped: usize,
}

pub(super) struct RuntimeCommitPlan {
    pub(super) entries: HashMap<String, PromptEntry>,
    pub(super) refreshed: usize,
    pub(super) removed: usize,
}

pub(super) struct PromptScanContext<'a> {
    pub(super) root: &'a Path,
    pub(super) incremental: bool,
    pub(super) registry: &'a PromptRegistry,
    pub(super) existing_names: &'a HashSet<String>,
}

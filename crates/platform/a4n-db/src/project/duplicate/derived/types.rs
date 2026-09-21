use super::*;

pub(super) struct DuplicatedDerivedSourceSnapshot {
    pub(super) snapshot: project_source_snapshot::Model,
    pub(super) chapter_ids: HashMap<i64, i64>,
}

pub(super) struct CopiedDerivedGenerationSpecs {
    pub(super) spec_ids: Vec<i64>,
    pub(super) spec_id_map: HashMap<i64, i64>,
    pub(super) segment_target_updates: Vec<(i64, i64)>,
}

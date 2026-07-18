use super::*;
use crate::project::duplicate::ProjectMediaCopyError;

pub trait DerivedProjectMediaFileCopier {
    fn copy_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError>;
}

pub(super) struct DuplicatedDerivedSourceSnapshot {
    pub(super) snapshot: project_source_snapshot::Model,
    pub(super) chapter_ids: HashMap<i64, i64>,
}

pub(super) struct CopiedDerivedGenerationSpecs {
    pub(super) spec_ids: Vec<i64>,
    pub(super) spec_id_map: HashMap<i64, i64>,
    pub(super) segment_target_updates: Vec<(i64, i64)>,
}

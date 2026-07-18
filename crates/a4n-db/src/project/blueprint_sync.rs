mod characters;
mod contracts;
mod identity;
mod records;
mod relationships;

pub use characters::sync_blueprint_characters;
pub use contracts::{
    BlueprintCharacterDraft, BlueprintIncrementalSyncStats, BlueprintRelationshipDraft,
    BlueprintSyncError, CHAPTERS_PER_PART, PART_OUTLINE_THRESHOLD,
    needs_part_outlines_for_chapters,
};
pub use identity::normalized_blueprint_identity;
pub use records::{
    blueprint_characters_for_project, blueprint_relationships_for_project,
    blueprint_version_by_project_version, blueprint_versions_by_project,
    novel_blueprint_for_project, update_blueprint_world_setting,
};
pub use relationships::sync_blueprint_relationships;

#[cfg(test)]
mod tests;

mod characters;
mod contracts;
mod identity;
mod records;
mod relationships;

pub use characters::{merge_blueprint_characters, sync_blueprint_characters};
pub use contracts::{
    BlueprintCharacterDraft, BlueprintIncrementalSyncStats, BlueprintRelationshipDraft,
    BlueprintSyncError,
};
pub use identity::normalized_blueprint_identity;
pub use records::{
    blueprint_characters_for_project, blueprint_relationships_for_project,
    blueprint_sources_need_review, blueprint_version_by_project_version,
    blueprint_versions_by_project, novel_blueprint_for_project,
};
pub use relationships::{merge_blueprint_relationships, sync_blueprint_relationships};

#[cfg(test)]
mod tests;

mod characters;
mod contracts;
mod identity;
mod record;
mod records;
mod relationships;

pub(crate) use record::blueprint_record;

pub(super) use characters::sync_blueprint_characters;
pub use characters::{edit_blueprint_characters, merge_blueprint_characters};
pub use contracts::{BlueprintCharacterDraft, BlueprintRelationshipDraft, BlueprintSyncError};
pub(crate) use identity::normalized_blueprint_identity;
pub use records::{
    blueprint_characters_for_project, blueprint_relationships_for_project,
    blueprint_sources_need_review, blueprint_version_by_project_version,
    blueprint_versions_by_project, novel_blueprint_for_project,
};
pub(super) use relationships::sync_blueprint_relationships;
pub use relationships::{edit_blueprint_relationships, merge_blueprint_relationships};

#[cfg(test)]
mod tests;

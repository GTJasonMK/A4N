mod counts;
mod persistence;
mod records;
mod sync;
mod types;

pub use counts::{scene_occurrence_count, scene_occurrence_counts_by_profile};
pub use records::{
    create_scene_profile_record, create_scene_profile_record_checked, delete_scene_profile_records,
    maybe_scene_profile_by_normalized_name, replace_scene_profile_record, scene_profile_by_id,
    scene_profiles_by_project, update_scene_profile_record_checked,
};
pub use sync::{
    delete_chapter_scene_sync_records, replace_chapter_scene_sync_records,
    replace_chapter_scene_sync_with_generation_run_records,
};
pub use types::{
    SceneEntityStateRecordInput, SceneOccurrenceRecordInput, SceneProfileDbError,
    SceneProfileDbResult, SceneProfileEvidenceRecordInput, SceneProfileRecordDraft,
    SceneProfileRecordInput, SceneSyncRecordInput, SceneSyncRecordResult,
    SceneTransitionRecordInput,
};

#[cfg(test)]
mod tests;

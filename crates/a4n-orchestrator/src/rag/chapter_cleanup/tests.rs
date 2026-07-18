use crate::project::ChapterDerivedCleanupArtifacts;

use super::plan::ChapterDerivedRagCleanupPlan;

#[test]
fn chapter_derived_rag_cleanup_plan_contains_only_rag_scopes() {
    let artifacts = ChapterDerivedCleanupArtifacts {
        stale_profile_ids: vec![1],
        stale_character_state_ids: vec![2],
        stale_attribute_event_ids: vec![3],
        stale_foreshadowing_ids: vec![4],
        changed_foreshadowing_ids: vec![5],
        media_file_paths: vec!["chapter/video.mp4".to_string()],
        scene_records_changed: true,
    };
    let plan = ChapterDerivedRagCleanupPlan::new(9, 10, 11, &artifacts);

    assert_eq!(plan.project_id, 9);
    assert_eq!(plan.chapter_id, 10);
    assert_eq!(plan.chapter_number, 11);
    assert_eq!(plan.stale_profile_ids, &[1]);
    assert_eq!(plan.stale_character_state_ids, &[2]);
    assert_eq!(plan.stale_attribute_event_ids, &[3]);
    assert_eq!(plan.stale_foreshadowing_ids, &[4]);
    assert_eq!(plan.changed_foreshadowing_ids, &[5]);
}

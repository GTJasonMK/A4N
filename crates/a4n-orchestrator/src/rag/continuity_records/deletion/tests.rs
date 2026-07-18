use crate::character::{CharacterProfileDeletionSummary, CharacterTimelineDeletionSummary};

use super::plan::CharacterRagDeletionPlan;

#[test]
fn profile_deletion_plan_includes_profile_state_and_attribute_event_scope() {
    let deletion = CharacterProfileDeletionSummary {
        project_id: 9,
        profile_id: 12,
        portrait_paths: vec!["portrait.png".to_string()],
        attribute_event_ids: vec![31, 32],
        state_index_ids: vec![41, 42],
    };
    let plan = CharacterRagDeletionPlan::for_profile(&deletion);

    assert_eq!(plan.project_id, 9);
    assert_eq!(plan.profile_id, Some(12));
    assert_eq!(plan.state_index_ids, &[41, 42]);
    assert_eq!(plan.attribute_event_ids, &[31, 32]);
}

#[test]
fn timeline_deletion_plan_excludes_profile_scope() {
    let deletion = CharacterTimelineDeletionSummary {
        attribute_event_ids: vec![31],
        state_index_ids: vec![41],
        ..Default::default()
    };
    let plan = CharacterRagDeletionPlan::for_timeline(9, &deletion);

    assert_eq!(plan.project_id, 9);
    assert_eq!(plan.profile_id, None);
    assert_eq!(plan.state_index_ids, &[41]);
    assert_eq!(plan.attribute_event_ids, &[31]);
}

use super::{RollbackImpactSummary, rules};
use a4n_core::state::rollback::CleanupAction;

fn rollback_test_impact(kind: &str, count: Option<i32>) -> RollbackImpactSummary {
    RollbackImpactSummary {
        kind: kind.to_string(),
        title: kind.to_string(),
        tables: Vec::new(),
        count,
        chapter_start: None,
        chapter_end: None,
        part_start: None,
        part_end: None,
        note: None,
    }
}

#[test]
fn rollback_impact_count_returns_zero_only_when_impact_is_absent() {
    let impacts = vec![rollback_test_impact("delete_chapters", None)];

    assert_eq!(
        rules::rollback_impact_count(&impacts, "delete_chapter_outlines"),
        Ok(0)
    );

    let err = rules::rollback_impact_count(&impacts, "delete_chapters")
        .expect_err("present impact count must be explicit");
    assert!(err.contains("回退影响 delete_chapters 缺少数量"));
}

#[test]
fn rollback_impact_count_accepts_explicit_count() {
    let impacts = vec![rollback_test_impact("delete_chapters", Some(3))];

    assert_eq!(
        rules::rollback_impact_count(&impacts, "delete_chapters"),
        Ok(3)
    );
}

#[test]
fn rollback_impact_query_maps_cleanup_actions() {
    let query = rules::rollback_impact_query(
        7,
        &[
            CleanupAction::DeleteChapters,
            CleanupAction::DeleteBlueprintVersions,
        ],
    );

    assert_eq!(query.project_id, 7);
    assert!(query.delete_chapters);
    assert!(!query.delete_chapter_outlines);
    assert!(!query.delete_part_outlines);
    assert!(query.delete_blueprint_versions);
}

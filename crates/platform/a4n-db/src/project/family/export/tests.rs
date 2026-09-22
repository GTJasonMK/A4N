use super::snapshots::collect_project_family_export_snapshot_ids;

#[test]
fn export_snapshot_ids_are_sorted_and_deduplicated() {
    assert_eq!(
        collect_project_family_export_snapshot_ids(&[5, 2, 5, 3], &[]),
        vec![2, 3, 5]
    );
}

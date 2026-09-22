use super::contracts::CharacterProfileRelatedCounts;

#[test]
fn empty_related_counts_produce_zero_timeline_rows() {
    assert_eq!(CharacterProfileRelatedCounts::default().timeline_rows(), 0);
}

#[test]
fn timeline_rows_exclude_snapshots_and_portraits() {
    let counts = CharacterProfileRelatedCounts {
        attribute_events: 2,
        character_states: 3,
        behaviors: 5,
        deletion_marks: 7,
        character_snapshots: 11,
        character_portraits: 13,
    };

    assert_eq!(counts.timeline_rows(), 17);
    assert_eq!(counts.character_snapshots, 11);
    assert_eq!(counts.character_portraits, 13);
}

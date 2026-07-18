use super::contracts::{PART_OUTLINE_THRESHOLD, needs_part_outlines_for_chapters};
use super::identity::{blueprint_relationship_identity_from_keys, normalized_blueprint_identity};

#[test]
fn normalized_identity_collapses_whitespace_and_case() {
    assert_eq!(
        normalized_blueprint_identity("  Alice\tSMITH  "),
        "alice smith"
    );
    assert_eq!(normalized_blueprint_identity(" \n "), "");
}

#[test]
fn relationship_identity_preserves_endpoint_order() {
    assert_eq!(
        blueprint_relationship_identity_from_keys("friend", "alice", "bob"),
        "friend|alice|bob"
    );
    assert_ne!(
        blueprint_relationship_identity_from_keys("friend", "alice", "bob"),
        blueprint_relationship_identity_from_keys("friend", "bob", "alice")
    );
}

#[test]
fn part_outline_threshold_is_strictly_greater_than_limit() {
    assert!(!needs_part_outlines_for_chapters(PART_OUTLINE_THRESHOLD));
    assert!(needs_part_outlines_for_chapters(PART_OUTLINE_THRESHOLD + 1));
}

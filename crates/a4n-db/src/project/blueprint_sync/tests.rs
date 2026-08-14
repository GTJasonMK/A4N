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

use a4n_domain::project::{
    blueprint_character_identity_keys, blueprint_relationship_identity_keys,
    normalized_blueprint_identity,
};

#[test]
fn relationship_type_uses_the_canonical_enum() {
    blueprint_relationship_identity_keys([("rival", "a", "b")])
        .expect("canonical relationship type");
    let error = blueprint_relationship_identity_keys([("rivalry", "a", "b")])
        .expect_err("custom type must fail");

    assert!(error.to_string().contains("rivalry"));
    assert!(
        error
            .to_string()
            .contains("family, friend, enemy, lover, rival, mentor, other")
    );
}

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
        blueprint_relationship_identity_keys([("friend", "alice", "bob")]).unwrap(),
        [("friend".to_string(), "alice".to_string(), "bob".to_string())]
    );
    assert_ne!(
        blueprint_relationship_identity_keys([("friend", "alice", "bob")]).unwrap(),
        blueprint_relationship_identity_keys([("friend", "bob", "alice")]).unwrap()
    );
}

#[test]
fn duplicate_identities_are_rejected_before_persistence() {
    assert!(blueprint_character_identity_keys(["Alice Smith", " alice\tSMITH "]).is_err());
    assert!(
        blueprint_relationship_identity_keys([
            ("friend", "Alice", "Bob"),
            ("friend", " alice ", "BOB")
        ])
        .is_err()
    );
    assert!(blueprint_character_identity_keys(["Alice", "Bob"]).is_ok());
}

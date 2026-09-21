#[test]
fn parses_canonical_blueprint_response() {
    let parsed = parse_blueprint_json(&valid_blueprint(), 30).expect("valid blueprint");

    assert_eq!(parsed.title, "星潮档案");
    assert_eq!(parsed.total_chapters, 30);
    assert_eq!(parsed.characters.len(), 3);
    assert_eq!(parsed.relationships[0].relationship_type, "rival");
}

#[test]
fn rejects_markdown_wrapped_json() {
    let raw = format!("```json\n{}\n```", valid_blueprint());

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_string_number_chapter_count() {
    let raw = valid_blueprint().replace("\"total_chapters\":30", "\"total_chapters\":\"30\"");

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

#[test]
fn rejects_mismatched_requested_chapter_count() {
    let err = parse_blueprint_json(&valid_blueprint(), 40).expect_err("mismatch");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn rejects_old_character_alias_fields() {
    let raw = valid_blueprint().replace(
        "\"background\":\"灾变幸存档案员\"",
        "\"identity\":\"灾变幸存档案员\"",
    );

    assert!(parse_blueprint_json(&raw, 30).is_err());
}

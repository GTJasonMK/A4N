#[test]
fn parse_character_discovery_output_rejects_unknown_evidence_anchor_id() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
            "evidence_anchor_id": 999
        }]
    })
    .to_string();

    let err = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("unknown evidence anchor id must fail");

    assert!(err.contains("evidence_anchor_id 无效"));
    assert!(err.contains("text_anchor_id 不在本次输入候选中"));
}

#[test]
fn parse_character_discovery_output_rejects_invalid_importance() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "importance": "lead",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string();
    let err = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("invalid importance must fail");

    assert!(err.contains("importance"));
}

#[test]
fn parse_character_discovery_output_rejects_missing_attribute_objects() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": [],
            "social_attributes": {},
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string();
    let err = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("attribute objects must be strict");

    assert!(err.contains("implicit_attributes"));
}

#[test]
fn parse_character_discovery_output_rejects_duplicate_alias_identity() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。沈先生回头。");
    let first_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let second_anchor_id = anchor_id_containing(&anchors, "沈先生回头");
    let content = json!({
        "characters": [
            {
                "name": "沈砚",
                "aliases": ["沈先生"],
                "importance": "primary",
                "role": "主角",
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {},
                "evidence_anchor_id": first_anchor_id
            },
            {
                "name": "沈先生",
                "aliases": [],
                "importance": "secondary",
                "role": "先生",
                "explicit_attributes": {},
                "implicit_attributes": {},
                "social_attributes": {},
                "evidence_anchor_id": second_anchor_id
            }
        ]
    })
    .to_string();
    let err = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("duplicate alias identity must fail");

    assert!(err.contains("重复"));
}

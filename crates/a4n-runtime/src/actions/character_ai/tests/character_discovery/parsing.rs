#[test]
fn parse_character_discovery_output_reports_truncated_json() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let err = parse_character_discovery_output(
            r#"{"characters":[{"name":"沈砚","aliases":[],"importance":"primary","role":"主角","explicit_attributes":{},"implicit_attributes":{},"social_attributes":{},"evidence_anchor_id":"#,
            &chapters,
            &anchors,
        )
        .expect_err("truncated discovery json should fail");

    assert!(err.contains("JSON 对象"));
    assert!(err.contains("输出被截断") || err.contains("max_tokens"));
}

#[test]
fn parse_character_discovery_output_accepts_canonical_object() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": ["沈先生", "沈砚"],
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {
              "identity": "书院弟子",
              "appearance": "青衫少年"
            },
            "implicit_attributes": {
              "cautious": "遇事先观察",
              "personality": "冷静谨慎"
            },
            "social_attributes": {
              "书院": "弟子"
            },
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string();
    let characters = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect("canonical discovery output should parse");

    assert_eq!(characters.len(), 1);
    assert_eq!(characters[0]["name"], json!("沈砚"));
    assert_eq!(characters[0]["aliases"], json!(["沈先生"]));
    assert_eq!(characters[0]["first_seen_chapter"], json!(1));
    assert_eq!(characters[0]["latest_seen_chapter"], json!(1));
    assert_eq!(
        characters[0]["explicit_attributes"]["identity"],
        json!("书院弟子")
    );

    let base = discovered_character_base_attributes(&characters[0])
        .expect("discovered character should map to base attributes");
    assert_eq!(base["explicit_attributes"]["role"], json!("主角"));
    assert_eq!(base["explicit_attributes"]["appearance"], json!("青衫少年"));
    assert_eq!(
        base["implicit_attributes"]["personality"],
        json!("冷静谨慎")
    );
    assert_eq!(base["social_attributes"]["书院"], json!("弟子"));
}

#[test]
fn parse_character_discovery_output_rejects_legacy_evidence_field() {
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
            "evidence": "沈砚推门而入"
        }]
    })
    .to_string();

    let err = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("legacy free-text evidence must fail");

    assert!(err.contains("evidence 已废弃"));
    assert!(err.contains("evidence_anchor_id"));
}

#[test]
fn parse_character_discovery_output_rejects_legacy_evidence_span_id() {
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
            "evidence_span_id": "ch9:s1:f1"
        }]
    })
    .to_string();

    let err = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("legacy evidence span id must fail");

    assert!(err.contains("evidence_span_id 已废弃"));
    assert!(err.contains("evidence_anchor_id"));
}

#[test]
fn parse_character_discovery_output_rejects_model_supplied_chapter_scope_fields() {
    let (chapters, anchors) = discovery_anchor_context("沈砚推门而入。");
    let evidence_anchor_id = anchor_id_containing(&anchors, "沈砚推门而入");
    let content = json!({
        "characters": [{
            "name": "沈砚",
            "aliases": [],
            "first_seen_chapter": 1,
            "importance": "primary",
            "role": "主角",
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
            "evidence_anchor_id": evidence_anchor_id
        }]
    })
    .to_string();

    let err = parse_character_discovery_output(&content, &chapters, &anchors)
        .expect_err("model supplied derived chapter field must fail");

    assert!(err.contains("未支持字段 `first_seen_chapter`"));
}

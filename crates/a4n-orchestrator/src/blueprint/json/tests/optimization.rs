#[test]
fn parse_blueprint_optimization_json_accepts_applicable_fields() {
    let value = parse_blueprint_optimization_json(
        r#"{"style":"冷峻写实","full_synopsis":"主角在压力中完成成长。"}"#,
    )
    .expect("valid optimization json");

    assert_eq!(value.style.as_deref(), Some("冷峻写实"));
    assert_eq!(
        value.full_synopsis.as_deref(),
        Some("主角在压力中完成成长。")
    );
}

#[test]
fn parse_blueprint_optimization_json_accepts_a_partial_character_patch() {
    let blueprint: serde_json::Value = serde_json::from_str(&valid_blueprint()).unwrap();
    let value = serde_json::json!({
        "characters": [blueprint["characters"][0].clone()]
    });

    let parsed = parse_blueprint_optimization_json(&value.to_string())
        .expect("optimization should accept one changed character");

    assert_eq!(parsed.characters.as_ref().map(Vec::len), Some(1));
}

#[test]
fn parse_blueprint_optimization_json_rejects_an_empty_character_patch() {
    let err = parse_blueprint_optimization_json(r#"{"characters":[]}"#)
        .expect_err("empty character patches must fail");

    assert!(err.to_string().contains("至少需要 1 个角色"));
}

#[test]
fn parse_blueprint_optimization_json_rejects_plain_text() {
    let err = parse_blueprint_optimization_json("整体结构不错，但建议加强冲突。")
        .expect_err("plain text should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn parse_blueprint_optimization_json_rejects_empty_applicable_fields() {
    let err = parse_blueprint_optimization_json(r#"{"style":" ","full_synopsis":null}"#)
        .expect_err("empty fields should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn parse_blueprint_optimization_json_rejects_unknown_field() {
    let err = parse_blueprint_optimization_json(
        r#"{"style":"冷峻写实","legacy_notes":"不要进入正式蓝图"}"#,
    )
    .expect_err("unknown optimization fields must fail");

    assert!(err.to_string().contains("未知字段 legacy_notes"));
}

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

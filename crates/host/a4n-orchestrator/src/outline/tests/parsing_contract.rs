use super::*;

#[test]
fn parse_outline_json_object_rejects_plain_text() {
    let err =
        parse_outline_json_object("这是一段说明", "outline").expect_err("plain text must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("输出预览"));
}

#[test]
fn parse_outline_json_object_rejects_json_array() {
    let err = parse_outline_json_object(r#"[{"title":"第一章"}]"#, "outline")
        .expect_err("array must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn parse_chapter_outline_batch_accepts_exact_canonical_chapters() {
    let parsed = parse_chapter_outline_batch(
            r#"{"chapters":[{"title":"暗线浮出","summary":"主角发现关键线索。"},{"title":"旧账重启","summary":"反派开始反制。"}]}"#,
            3,
            2,
        )
        .expect("canonical chapter batch should parse");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0]["chapter_number"], json!(3));
    assert_eq!(parsed[1]["title"], json!("旧账重启"));
}

#[test]
fn parse_chapter_outline_batch_requires_array_structured_fields() {
    let parsed = parse_chapter_outline_batch(
            r#"{"chapters":[{"title":"暗线浮出","summary":"主角发现关键线索。","beats":["发现线索"],"key_scenes":["暗巷追踪"],"foreshadowing_setup":["旧徽记"],"foreshadowing_payoff":["旧案回响"],"characters_involved":["主角"]}]}"#,
            3,
            1,
        )
        .expect("array structured fields should parse");

    assert_eq!(parsed[0]["beats"], json!(["发现线索"]));
    assert_eq!(parsed[0]["characters_involved"], json!(["主角"]));

    for (field, value) in [
        ("beats", r#""发现线索""#),
        ("key_scenes", r#"{"title":"暗巷追踪"}"#),
        ("foreshadowing_setup", "null"),
        ("foreshadowing_payoff", r#""旧案回响""#),
        ("characters_involved", r#"{"name":"主角"}"#),
    ] {
        let payload =
            format!(r#"{{"chapters":[{{"title":"一","summary":"开端","{field}":{value}}}]}}"#);
        let err = parse_chapter_outline_batch(&payload, 1, 1)
            .expect_err("structured field with wrong type must fail");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(
            err.to_string().contains(field),
            "error should mention {field}: {err}"
        );
        assert!(err.to_string().contains("必须是数组"));
    }
}

#[test]
fn parse_chapter_outline_batch_rejects_unknown_wrapper() {
    let err = parse_chapter_outline_batch(r#"{"items":[{"title":"一","summary":"开端"}]}"#, 1, 1)
        .expect_err("unknown wrapper must fail");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("items"));
}

#[test]
fn parse_chapter_outline_batch_requires_exact_count_and_order() {
    let count_err =
        parse_chapter_outline_batch(r#"{"chapters":[{"title":"一","summary":"开端"}]}"#, 1, 2)
            .expect_err("missing chapter must fail");
    assert!(count_err.to_string().contains("必须包含 2 个章节"));

    let parsed = parse_chapter_outline_batch(
        r#"{"chapters":[{"title":"一","summary":"开端"},{"title":"二","summary":"后续"}]}"#,
        1,
        2,
    )
    .expect("chapter numbers should be derived by order");
    assert_eq!(parsed[0]["chapter_number"], json!(1));
    assert_eq!(parsed[1]["chapter_number"], json!(2));
}

#[test]
fn parse_chapter_outline_batch_rejects_model_owned_numbers_and_unknown_fields() {
    let model_owned_number = parse_chapter_outline_batch(
        r#"{"chapters":[{"chapter_number":"1","title":"一","summary":"开端"}]}"#,
        1,
        1,
    )
    .expect_err("model-owned chapter number must fail");
    assert!(model_owned_number.to_string().contains("chapter_number"));

    let unknown = parse_chapter_outline_batch(
        r#"{"chapters":[{"title":"一","summary":"开端","chapter_arcs":[]} ]}"#,
        1,
        1,
    )
    .expect_err("unknown item field must fail");
    assert!(unknown.to_string().contains("chapter_arcs"));
}

#[test]
fn parse_chapter_outline_batch_rejects_non_positive_word_count_target() {
    let zero = parse_chapter_outline_batch(
        r#"{"chapters":[{"title":"一","summary":"开端","word_count_target":0}]}"#,
        1,
        1,
    )
    .expect_err("zero word count target must fail");
    assert!(zero.to_string().contains("word_count_target"));

    let negative = parse_chapter_outline_batch(
        r#"{"chapters":[{"title":"一","summary":"开端","word_count_target":-3000}]}"#,
        1,
        1,
    )
    .expect_err("negative word count target must fail");
    assert!(negative.to_string().contains("word_count_target"));
}

#[test]
fn parse_part_outline_single_response_accepts_exact_canonical_object() {
    let parsed = parse_part_outline_single_response(
            r#"{"title":"暗潮卷","summary":"主线推进并进入第二阶段。","theme":"代价","key_events":["旧敌回归","盟友背叛","真相浮出"],"character_focus":{"主角":"从迟疑转为主动"}}"#,
            2,
            11,
            20,
        )
        .expect("canonical part outline should parse");

    assert_eq!(parsed["part_number"], json!(2));
    assert_eq!(parsed["character_focus"]["主角"], json!("从迟疑转为主动"));
}

#[test]
fn parse_part_outline_single_response_rejects_wrappers_and_unknown_fields() {
    let wrapped =
        parse_part_outline_single_response(r#"{"parts":[{"title":"暗潮卷"}]}"#, 2, 11, 20)
            .expect_err("wrapped parts must fail");
    assert!(wrapped.to_string().contains("parts"));

    let unknown = parse_part_outline_single_response(
            r#"{"title":"暗潮卷","summary":"主线推进","theme":"代价","key_events":["旧敌回归"],"character_focus":{"主角":"成长"},"unexpected_field":{}}"#,
            2,
            11,
            20,
        )
        .expect_err("unknown field must fail");
    assert!(unknown.to_string().contains("unexpected_field"));
}

#[test]
fn parse_part_outline_single_response_rejects_model_owned_numbers() {
    let err = parse_part_outline_single_response(
            r#"{"part_number":"2","title":"暗潮卷","summary":"主线推进","theme":"代价","key_events":["旧敌回归","盟友背叛","真相浮出"],"character_focus":{"主角":"成长"}}"#,
            2,
            11,
            20,
        )
        .expect_err("model-owned part number must fail");

    assert!(err.to_string().contains("part_number"));
}

#[test]
fn parse_part_outline_single_response_rejects_too_few_key_events() {
    let err = parse_part_outline_single_response(
            r#"{"title":"暗潮卷","summary":"主线推进","theme":"代价","key_events":["旧敌回归"],"character_focus":{"主角":"成长"}}"#,
            2,
            11,
            20,
        )
        .expect_err("key_events must have at least three items");

    assert!(err.to_string().contains("key_events"));
}

#[test]
fn parse_beat_sheet_json_rejects_blank_non_json_array_and_empty_object() {
    for input in ["", "不是 JSON", r#"["起", "承"]"#, "{}"] {
        let err = parse_beat_sheet_json(input).expect_err("beat sheet must be a JSON object");
        assert!(matches!(err, AppError::InvalidArgument(_)));
    }
}

#[test]
fn part_outline_single_rejects_wrapped_parts_field() {
    let value = json!({"parts": [{"part_number": 1}]});
    let err = reject_keys(&value, &["parts"], "part_outline_single")
        .expect_err("wrapped parts must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn part_outline_single_rejects_unknown_fields() {
    let value = json!({
        "title": "星火初燃",
        "summary": "主线推进",
        "theme": "启程",
        "key_events": ["启程"],
        "character_focus": {"主角": "成长"},
        "ending_hook": "未支持字段"
    });
    let err = reject_unknown_keys(
        &value,
        &["title", "summary", "theme", "key_events", "character_focus"],
        "part_outline_single",
    )
    .expect_err("unknown ending_hook must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn required_string_rejects_blank_values() {
    let err = required_string(&json!({"title": "  "}), "title").expect_err("blank title must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn optional_i32_rejects_non_integer_values() {
    let err = optional_i32(&json!({"word_count_target": "3000"}), "word_count_target")
        .expect_err("string integer must fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

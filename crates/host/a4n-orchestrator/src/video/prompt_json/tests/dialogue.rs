#[test]
fn parses_video_prompt_response_with_empty_dialogue_beats() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"] = json!([]);
    let raw = response_from_shots(vec![shot]);

    let shots = parse_with_valid_segments(&raw).expect("empty dialogue beats are valid");

    assert!(
        shots[0]
            .dialogue_beats
            .as_array()
            .is_some_and(Vec::is_empty)
    );
}

#[test]
fn rejects_video_prompt_response_missing_dialogue_cues() {
    let mut shot = valid_shot_value(1);
    shot.as_object_mut()
        .expect("shot object")
        .remove("dialogue_cues");
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("dialogue_cues must be explicit, even when empty");

    assert!(err.to_string().contains("missing field `dialogue_cues`"));
}

#[test]
fn parses_video_prompt_response_with_long_dialogue_line() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["line"] = json!(
        "这是一句被错误完整复制进视频提示词的超长对白，超过四十个中文字符，会让视频模型误以为需要生成连续字幕或复杂口播"
    );
    let raw = response_from_shots(vec![shot]);

    let shots = parse_with_valid_segments(&raw).expect("dialogue length is a prompt quality rule");

    assert!(
        shots[0].dialogue_beats[0]["line"]
            .as_str()
            .is_some_and(|line| line.contains("超长对白"))
    );
}

#[test]
fn derives_dialogue_source_span_ids_from_current_segment() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);

    let shots = parse_with_valid_segments(&raw)
        .expect("dialogue source spans should be derived from current segment");

    assert_eq!(
        shots[0].dialogue_beats[0]["source_span_ids"],
        json!(["ch1:s1:f1"])
    );
}

#[test]
fn revalidates_internal_dialogue_source_span_ids_from_stored_prompt() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);
    let mut shots = parse_with_valid_segments(&raw)
        .expect("model output should be normalized with backend source spans");
    let segments = valid_segments();
    let segment_by_number = segment_map(&segments);
    let segment = segment_by_number.get(&1).expect("segment 1");

    let shot = validate_video_prompt_shot_against_segment(
        shots.remove(0),
        segment,
        PromptOutputLanguage::ZhCn,
    )
    .expect("stored normalized video prompt should be revalidatable");

    assert_eq!(
        shot.dialogue_beats[0]["source_span_ids"],
        json!(["ch1:s1:f1"])
    );
}

#[test]
fn rejects_model_owned_dialogue_source_span_id() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["source_span_id"] = json!("ch1:s1:f2");
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("dialogue source_span_id is an internal backend-derived field");

    assert!(
        err.to_string()
            .contains("视频提示词 dialogue_cues[1] 不要输出 source_span_id/source_span_ids")
    );
}

#[test]
fn rejects_model_owned_dialogue_source_span_ids() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["source_span_ids"] = json!(["ch1:s1:f1"]);
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("dialogue source_span_ids is an internal backend-derived field");

    assert!(
        err.to_string()
            .contains("视频提示词 dialogue_cues[1] 不要输出 source_span_id/source_span_ids")
    );
}

#[test]
fn rejects_dialogue_extra_field() {
    let mut shot = valid_shot_value(1);
    shot["dialogue_cues"][0]["dialogue_internal_note"] = json!("模型额外字段");
    let raw = response_from_shots(vec![shot]);

    let err =
        parse_with_valid_segments(&raw).expect_err("dialogue beats must be a strict whitelist");

    assert!(
        err.to_string()
            .contains("视频提示词 dialogue_cues[1].dialogue_internal_note 不是允许字段")
    );
}

#[test]
fn rejects_video_prompt_response_with_model_owned_system_fields() {
    let mut shot = valid_shot_value(1);
    shot["temporal_beats"] = json!([
        {"start_sec": 0, "end_sec": 6, "camera": "宽镜头", "subject_action": "主角移动", "continuity_target": "书桌旁"}
    ]);
    let raw = response_from_shots(vec![shot]);

    assert!(parse_with_valid_segments(&raw).is_err());
}

#[test]
fn rejects_video_prompt_response_with_raw_newline_inside_string() {
    let raw = response_from_shots(vec![valid_shot_value(1)]).replace(
        "[主体定义与素材绑定]\\n参考图 1",
        "[主体定义与素材绑定]\n参考图 1",
    );

    assert!(parse_with_valid_segments(&raw).is_err());
}

#[test]
fn rejects_invalid_json() {
    assert!(parse_with_valid_segments("not json").is_err());
}

#[test]
fn rejects_markdown_fenced_json() {
    let raw = format!(
        "```json\n{}\n```",
        response_from_shots(vec![valid_shot_value(1)])
    );
    assert!(parse_with_valid_segments(&raw).is_err());
}

#[test]
fn rejects_missing_required_fields() {
    let raw = r#"{"shots":[{"title":"Opening"}]}"#;
    assert!(parse_with_valid_segments(raw).is_err());
}

#[test]
fn rejects_video_prompt_with_model_owned_shot_number() {
    let mut shot = valid_shot_value(1);
    shot["shot_number"] = json!(1);
    let raw = response_from_shots(vec![shot]);
    let err = parse_with_valid_segments(&raw).expect_err("shot_number must be backend-derived");
    assert!(err.to_string().contains("unknown field `shot_number`"));
}

#[test]
fn rejects_video_prompt_when_shot_number_cannot_be_derived() {
    let raw = response_from_shots(vec![valid_shot_value(1), valid_shot_value(1)]);
    let err = parse_with_valid_segments(&raw)
        .expect_err("multiple returned shots cannot map to one segment");
    assert!(err.to_string().contains("无法从当前批次和片段数量唯一推导"));
}

#[test]
fn parses_chat_video_prompt_with_source_span_ids() {
    let raw = response_from_shots(vec![valid_chat_video_shot_value(1)]);

    let shots = parse_chat_with_valid_segments(&raw).expect("valid chat prompt");

    assert_eq!(shots[0].shot_number, 1);
    assert_eq!(shots[0].chat_video["source_span_ids"][0], "ch1:s1:f1");
    assert_eq!(shots[0].reference_image_ids, vec![1, 2]);
    assert_eq!(shots[0].chat_video["input_images"][1]["image_id"], 2);
    let action = shots[0].chat_video["timeline"][0]["action"]
        .as_str()
        .expect("chat timeline action should be text");
    assert!(
        !action.contains('{') && !action.contains("movement_path"),
        "chat timeline should not dump performance_plan JSON: {action}"
    );
    assert!(
        shots[0].chat_video.get("performance_plan").is_none(),
        "chat_video should not duplicate the full performance_plan object"
    );
}

#[test]
fn rejects_chat_video_prompt_without_no_background_music_rule() {
    let mut shot = valid_chat_video_shot_value(1);
    let prompt = shot["chat_prompt"]
        .as_str()
        .unwrap()
        .replace("，不要配背景音乐", "");
    shot["chat_prompt"] = json!(prompt);
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt should forbid missing background music rule");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("不要配背景音乐")
                || message.contains("背景音乐")
    ));
}

#[test]
fn rejects_chat_video_prompt_when_no_background_music_rule_is_outside_avoidance_section() {
    let mut shot = valid_chat_video_shot_value(1);
    let prompt = shot["chat_prompt"]
        .as_str()
        .unwrap()
        .replace(
            "主角进入档案室并保持谨慎。",
            "主角进入档案室并保持谨慎，不要配背景音乐。",
        )
        .replace("，不要配背景音乐", "");
    shot["chat_prompt"] = json!(prompt);
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("no-background-music rule must appear in the avoidance section");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("限制与避免项")
                && (message.contains("不要配背景音乐") || message.contains("背景音乐"))
    ));
}

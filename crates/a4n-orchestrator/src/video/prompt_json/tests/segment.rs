#[test]
fn parses_video_segment_plan_with_intent_arc_and_material_coverage() {
    let plan = valid_segment_plan_value();

    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");

    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].video_intent["hook"], "主角第一次进入异常档案室");
    assert_eq!(segments[0].emotional_arc["function"], "场景建立");
    assert_eq!(
        segments[0].performance_plan["movement_path"],
        "主角从档案室门口偏左进入，沿书架左侧向画面深处书桌慢走，并在书桌旁停住。"
    );
    assert!(segments[0].performance_plan["action_script"].is_array());
    assert!(segments[0].material_coverage["risk_note"].is_string());
    assert!(
        segments[0].reference_image_ids.is_empty(),
        "片段规划阶段不绑定具体参考图"
    );
}

#[test]
fn rejects_video_segment_plan_with_duplicate_object_field_hint() {
    let raw = r#"{
      "segments": [
        {
          "source_shot_id": 1,
          "covered_beat_ids": [1],
          "title": "开场",
          "scene_summary": "主角进入档案室",
          "blocking_summary": "主角从档案室门口走向书桌",
          "spatial_anchors": [],
          "emotion_action_goals": [],
          "video_intent": {},
          "emotional_arc": {"start": "谨慎"},
          "emotional_arc": {"end": "戒备"},
          "performance_plan": {},
          "material_coverage": {}
        }
      ]
    }"#;

    let err = parse_video_segment_plan_input_values(raw, "视频分段规划 镜头 #2")
        .expect_err("duplicate model-owned fields must be rejected");
    let message = err.to_string();

    assert!(message.contains("duplicate field `emotional_arc`"));
}

#[test]
fn rejects_missing_video_segment_plan_performance_plan() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]
        .as_object_mut()
        .expect("segment should be object")
        .remove("performance_plan");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail when performance_plan is missing");

    assert!(err.to_string().contains("missing field `performance_plan`"));
}

#[test]
fn rejects_segment_plan_time_block_empty_camera() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["performance_plan"]["time_blocks"][1]["camera"] = json!("   ");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail when performance_plan time block fields are empty");
    assert!(
        err.to_string()
            .contains("视频分段规划 performance_plan.time_blocks[2].camera 不能为空")
    );
}

#[test]
fn rejects_segment_plan_time_blocks_exceeding_duration() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["performance_plan"]["time_blocks"][1]["end_sec"] = json!(8);

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail before deriving video prompt temporal_beats");

    assert!(
        err.to_string().contains(
            "视频分段规划 performance_plan.time_blocks[2].end_sec 不能超过 duration_secs"
        )
    );
}

#[test]
fn rejects_segment_plan_action_script_empty_hands() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["performance_plan"]["action_script"][0]["hands"] = json!("   ");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must fail when action_script hands is empty");

    assert!(
        err.to_string()
            .contains("视频分段规划 performance_plan.action_script[1].hands 不能为空")
    );
}

#[test]
fn rejects_stored_video_segment_plan_with_legacy_reference_image_ids() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["reference_image_ids"] = json!([3, 9]);

    let err = parse_stored_video_segment_plan_for_reuse(
        &stored.to_string(),
        &valid_source_spans(),
        "16:9",
    )
    .expect_err("old segment-level reference bindings must not be reused");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("reference_image_ids")
                && message.contains("片段规划不再绑定具体素材")
    ));
}

#[test]
fn rejects_retargeting_stored_video_segment_plan_for_resume() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["duration_secs"] = json!(12);
    stored["segments"][0]["aspect_ratio"] = json!("4:3");
    stored["segments"][0]["reference_image_ids"] = json!([3, 9]);

    let err = parse_stored_video_segment_plan_for_reuse(
        &stored.to_string(),
        &valid_source_spans(),
        "9:16",
    )
    .expect_err("stored segment plan must not be retargeted to current options");

    assert!(
        err.to_string()
            .contains("aspect_ratio 必须等于当前请求 9:16")
    );
}

#[test]
fn reuses_stored_video_segment_plan_with_different_duration() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["duration_secs"] = json!(12);
    stored["segments"][0]["performance_plan"]["time_blocks"][0]["end_sec"] = json!(6);
    stored["segments"][0]["performance_plan"]["time_blocks"][1]["start_sec"] = json!(6);
    stored["segments"][0]["performance_plan"]["time_blocks"][1]["end_sec"] = json!(12);

    let parsed = parse_stored_video_segment_plan_for_reuse(
        &stored.to_string(),
        &valid_source_spans(),
        "16:9",
    )
    .expect("duration is planning context and should not block segment plan reuse");

    assert_eq!(parsed[0].duration_secs, 12);
}

#[test]
fn rejects_reusing_stored_video_segment_plan_with_stale_source_span_ids() {
    let plan = valid_segment_plan_value();
    let segments = parse_valid_segment_plan(&plan).expect("segment plan should parse");
    let mut stored = json!({ "segments": segments });
    stored["segments"][0]["source_span_ids"] = json!(["old:span"]);

    let err = parse_stored_video_segment_plan_for_reuse(
        &stored.to_string(),
        &valid_source_spans(),
        "16:9",
    )
    .expect_err("stale source span ids must fail instead of being retargeted");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("source_span_ids 包含不在本次输入候选中的 ID：old:span")
    ));
}

#[test]
fn rejects_video_segment_plan_without_directing_fields() {
    let mut plan = valid_segment_plan_value();
    let segment = plan["segments"][0].as_object_mut().expect("segment object");
    segment.remove("video_intent");
    segment.remove("emotional_arc");
    segment.remove("material_coverage");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan must include current directing fields");

    assert!(err.to_string().contains("video_intent"));
}

#[test]
fn rejects_video_segment_plan_with_incomplete_directing_fields() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["video_intent"]
        .as_object_mut()
        .expect("video_intent object")
        .remove("visual_priority");
    plan["segments"][0]["emotional_arc"]["end"] = json!("   ");

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment plan directing fields must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 video_intent.visual_priority 必须是非空字符串")
    );
}

#[test]
fn rejects_video_segment_plan_with_incomplete_spatial_anchor() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["spatial_anchors"][0]
        .as_object_mut()
        .expect("spatial anchor object")
        .remove("usage");

    let err = parse_valid_segment_plan(&plan).expect_err("spatial anchors must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 spatial_anchors[1].usage 必须是非空字符串")
    );
}

#[test]
fn rejects_video_segment_plan_with_incomplete_emotion_action_goal() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["emotion_action_goals"][0]["visible_action"] = json!("");

    let err =
        parse_valid_segment_plan(&plan).expect_err("emotion action goals must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 emotion_action_goals[1].visible_action 不能为空")
    );
}

#[test]
fn rejects_video_segment_plan_with_empty_material_coverage() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["material_coverage"] = json!({});

    let err = parse_valid_segment_plan(&plan).expect_err("material coverage is required");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("material_coverage") && message.contains("不能为空")
    ));
}

#[test]
fn rejects_video_segment_plan_with_incomplete_material_coverage() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["material_coverage"]
        .as_object_mut()
        .expect("material coverage object")
        .remove("risk_note");

    let err =
        parse_valid_segment_plan(&plan).expect_err("material coverage must match the contract");

    assert!(
        err.to_string()
            .contains("视频分段规划 material_coverage.risk_note 必须是非空字符串")
    );
}

#[test]
fn rejects_video_segment_plan_with_material_coverage_reference_ids() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["material_coverage"]["covered_by_reference_image_ids"] =
        json!([{"image_id": 3, "entity": "主角"}]);

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment material coverage must not bind concrete images");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("covered_by_reference_image_ids")
                && message.contains("不绑定具体图片 ID")
    ));
}

#[test]
fn rejects_video_segment_plan_with_model_owned_reference_image_ids() {
    let mut plan = valid_segment_plan_value();
    plan["segments"][0]["reference_image_ids"] = json!([3, 9]);

    let err = parse_valid_segment_plan(&plan)
        .expect_err("segment-level references must not be output by the model");

    assert!(
        err.to_string()
            .contains("unknown field `reference_image_ids`")
    );
}

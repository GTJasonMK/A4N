#[test]
fn rejects_video_prompt_response_wrapped_in_single_json_fence() {
    let raw = format!(
        "```json\n{}\n```",
        response_from_shots(vec![valid_shot_value(1)])
    );
    let err = parse_with_valid_segments(&raw)
        .expect_err("markdown-fenced JSON must violate the strict response contract");

    let message = err.to_string();
    assert!(message.contains("严格 JSON 对象"));
    assert!(message.contains("Markdown 代码围栏"));
    assert!(message.contains("不能用 ``` 或 ```json 包裹"));
}

#[test]
fn rejects_video_prompt_motion_prompt_with_duplicate_shot_storyboard() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[镜头分镜]\n镜头 1：主角在门口停顿观察。\n镜头 2：主角沿书架走向书桌。\n\n[运镜与动作]\n镜头从门口宽景缓慢推进。\n\n[对白与表演]\n无对白。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("motion_prompt must not repeat the visual prompt timeline");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("motion_prompt")
                && message.contains("不应重复镜头分镜")
    ));
}

#[test]
fn rejects_video_prompt_motion_prompt_with_deprecated_continuity_section() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口宽景缓慢推进，主角脚步放慢，肩膀收紧，视线从书架阴影移向深处书桌。\n\n[对白与表演]\n主角低声短句“这里不对劲”，嘴唇微动，脚步放慢。\n\n[连续性要求]\n旧版连续性尾态不应再写入 motion_prompt。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("motion_prompt must reject deprecated continuity sections");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("motion_prompt")
                && message.contains("旧版视频提示词章节 [连续性要求]")
    ));
}

#[test]
fn rejects_video_prompt_with_legacy_temporal_storyboard_section() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角一致性，参考图 2 用于档案大厅空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口进入并靠近书桌，结尾停在书桌旁保持戒备。\n\n[时序分镜]\n总时长：6秒。0-3秒：主角观察。3-6秒：主角走向书桌。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("legacy visual_prompt section must not satisfy the current contract");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("旧版视频提示词章节 [时序分镜]")
    ));
}

#[test]
fn rejects_video_prompt_without_required_visual_sections() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角一致性，参考图 2 用于档案大厅空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口进入并靠近书桌，结尾停在书桌旁保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("visual_prompt must keep the six-section API contract");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("缺少必需章节 [镜头分镜]")
    ));
}

#[test]
fn rejects_video_prompt_without_required_motion_sections() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口宽景缓慢向前推进，只保留一个稳定推进运动，主角脚步放慢，肩膀收紧，视线从书架阴影移向深处书桌。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("motion_prompt must keep the two-section API contract");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("motion_prompt")
                && message.contains("缺少必需章节 [对白与表演]")
    ));
}

#[test]
fn rejects_video_prompt_with_upload_status_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n无输入首帧。本段仅依赖文字描述保持角色形象一致。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛，结尾交给下一段的状态是他停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景建立门框、书架和书桌的空间关系，主角在画面左侧停半拍观察，视线扫过书架阴影。\n镜头 2：平视中景缓慢推进，主角沿书架左侧向书桌慢走，最后在书桌旁停住并保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("API prompt must not write upload-status placeholders");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("上传状态说明")
                && message.contains("无输入首帧")
    ));
}

#[test]
fn rejects_video_prompt_with_tail_frame_upload_status_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角脸部、发型和服装一致性；没有输入尾帧，本段结尾按文字描述生成。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛，结尾交给下一段的状态是他停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景建立门框、书架和书桌的空间关系，主角在画面左侧停半拍观察，视线扫过书架阴影。\n镜头 2：平视中景缓慢推进，主角沿书架左侧向书桌慢走，最后在书桌旁停住并保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("API prompt must not write tail-frame upload-status placeholders");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("上传状态说明")
                && message.contains("没有输入尾帧")
    ));
}

#[test]
fn parses_video_prompt_with_gore_detail_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 1 用于主角脸部、发型、年龄感和深色长外套一致性；参考图 2 用于档案大厅入口、木质书架、铜制吊灯和中央书桌的空间基准。\n\n[片段内容以及前因后果]\n主角从档案室门口偏左的起点进入，目标是靠近深处书桌并确认异常气氛；画面里血从指缝渗出、血滴落和血迹扩散都应转成可生成的非血腥痕迹与环境压迫感，结尾停在书桌旁保持戒备。\n\n[镜头分镜]\n镜头 1：门口宽中景保持门框、书架和书桌同框，主角站在画面左侧先停半拍观察，视线从书架阴影扫到深处书桌，空间保持安静压迫。\n镜头 2：平视中景缓慢推进，主角沿左侧书架向书桌慢走，肩膀略收紧，看见非血腥痕迹后停顿，最后在书桌旁保持戒备。\n\n[画面风格描述]\n电影感写实，暖色窗光从画面左侧斜照，阴影落向右后方，构图稳定并突出门、书架和书桌的空间关系。\n\n[连续性与结尾状态]\n人物始终保持画面左侧，门、书桌和书架的位置关系不变，结尾停在书桌旁，表情戒备，左侧窗光方向保持连续。\n\n[限制与避免项]\n不要配背景音乐，不要字幕、水印、换脸、空间跳变、镜头过快或角色服装变化。"
    );
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从首帧的门口宽景缓慢向前推进，主角先在门口停顿观察，再沿书架左侧向书桌移动，脚步放慢并保持戒备。\n\n[对白与表演]\n无对白，表演重点是谨慎观察、呼吸压低和看见非血腥痕迹后的短暂停顿。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_with_valid_segments(&raw)
        .expect("gore wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn rejects_chat_prompt_with_internal_image_id_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图 image_id=9 用于场景参考。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt text must not expose internal image ids");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("chat_prompt")
                && message.contains("内部图片 ID")
    ));
}

#[test]
fn rejects_chat_prompt_with_upload_status_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n本段无上传图片，按文字描述生成。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt must describe the actual start state instead of upload status");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("chat_prompt")
                && message.contains("上传状态说明")
                && message.contains("无上传图片")
    ));
}

#[test]
fn rejects_chat_prompt_with_reference_image_upload_status_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n无参考图，按文字描述建立档案室门口、书桌和书架的空间关系。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_chat_with_valid_segments(&raw)
        .expect_err("chat prompt must describe the actual start state instead of upload status");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("chat_prompt")
                && message.contains("上传状态说明")
                && message.contains("无参考图")
    ));
}

#[test]
fn parses_chat_prompt_with_self_harm_intent_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。\n[片段内容以及前因后果]\n主角进入档案室并为后续自残动作做铺垫。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_chat_with_valid_segments(&raw)
        .expect("self-harm wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_chat_prompt_with_gore_detail_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎，画面用非血腥痕迹和环境压迫感替代血从指缝渗出、血滴落和血迹扩散。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌，看见非血腥痕迹后停顿并在结尾保持戒备。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_chat_with_valid_segments(&raw)
        .expect("gore wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_chat_prompt_with_explicit_self_harm_execution_in_direct_text() {
    let mut shot = valid_chat_video_shot_value(1);
    shot["chat_prompt"] = json!(
        "[主体定义与素材绑定]\n画面从档案室门口开始，主角站在门口偏左，暖色窗光从左侧斜照，远处书桌和书架作为空间锚点。\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并把铁片刺入眼睛。\n[画面风格描述]\n电影感写实，暖色窗光。\n[连续性与结尾状态]\n结尾停在书桌旁并保持戒备。\n[限制与避免项]\n避免字幕、水印、换脸、空间跳变，不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_chat_with_valid_segments(&raw).expect(
        "self-harm execution wording is left to prompt optimization instead of hard validation",
    );

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_video_prompt_response_without_model_owned_system_fields() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);
    let shots = parse_with_valid_segments(&raw).expect("valid response");
    assert_eq!(shots.len(), 1);
    assert_eq!(shots[0].shot_number, 1);
    assert!(shots[0].visual_prompt.starts_with("[主体定义与素材绑定]"));
    assert_eq!(shots[0].duration_secs, 6);
    assert_eq!(shots[0].aspect_ratio, "16:9");
    assert_eq!(shots[0].reference_image_ids, vec![1, 2]);
}

#[test]
fn derived_prompt_fields_use_concise_performance_summary() {
    let raw = response_from_shots(vec![valid_shot_value(1)]);
    let shots = parse_with_valid_segments(&raw).expect("valid response");
    let movement = shots[0]
        .blocking_plan
        .get("movement")
        .and_then(Value::as_str)
        .expect("blocking movement should be text");

    assert!(movement.contains("从档案室门口偏左向画面深处书桌慢走"));
    assert!(movement.contains("肩膀略收紧"));
    assert!(
        !movement.contains('{') && !movement.contains("movement_path"),
        "movement should not dump performance_plan JSON: {movement}"
    );
    assert!(
        shots[0].blocking_plan.get("performance_plan").is_none(),
        "blocking_plan should not duplicate the full performance_plan object"
    );
    assert_eq!(shots[0].start_state["character"], "主角");
    for (field, value) in [
        ("start_state.pose", &shots[0].start_state["pose"]),
        (
            "transition_in.first_beat_action",
            &shots[0].transition_in["first_beat_action"],
        ),
        (
            "continuity_check.emotion",
            &shots[0].continuity_check["emotion"],
        ),
        (
            "reference_binding.coverage_check",
            &shots[0].reference_binding["coverage_check"],
        ),
    ] {
        let text = value.as_str().expect("derived field should be text");
        assert!(
            !text.contains('{') && !text.contains("\":"),
            "{field} should be concise natural language, got: {text}"
        );
    }
}

#[test]
fn parses_video_prompt_response_with_empty_shot_specific_negative_prompt() {
    let mut shot = valid_shot_value(1);
    shot["negative_prompt"] = json!("");
    let raw = response_from_shots(vec![shot]);

    let shots = parse_with_valid_segments(&raw).expect("empty shot-specific negative is valid");

    assert_eq!(shots[0].negative_prompt, "");
}

#[test]
fn rejects_video_prompt_with_internal_image_id_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["visual_prompt"] = json!(
        "[主体定义与素材绑定]\n参考图（image_id=9）约束档案大厅空间，主角站在门口偏左，木质书架和中央书桌保持稳定关系，暖色窗光从画面左侧斜照，画面从门口宽景开始并在书桌旁稳定收束，整体保持连续的电影感写实风格。\n\n[片段内容以及前因后果]\n主角进入档案室并保持谨慎。\n\n[镜头分镜]\n镜头 1：门口宽中景建立书架和书桌关系，主角站在门口偏左并停顿观察。\n镜头 2：平视中景缓慢推进，主角沿书架走向书桌并在结尾停住。\n\n[画面风格描述]\n电影感写实。\n\n[连续性与结尾状态]\n结尾停在书桌旁。\n\n[限制与避免项]\n不要配背景音乐。"
    );
    let raw = response_from_shots(vec![shot]);

    let err = parse_with_valid_segments(&raw)
        .expect_err("direct prompt text must not expose internal image ids");

    assert!(matches!(
        err,
        AppError::InvalidArgument(message)
            if message.contains("visual_prompt")
                && message.contains("内部图片 ID")
    ));
}

#[test]
fn parses_video_prompt_with_self_harm_intent_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口中景缓慢推近，主角站在门口，手指轻触门框并深呼吸，表情从迟疑转为决绝。\n\n[对白与表演]\n无对白。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_with_valid_segments(&raw)
        .expect("self-harm wording is left to prompt optimization instead of hard validation");

    assert_eq!(rows.len(), 1);
}

#[test]
fn parses_video_prompt_with_explicit_self_harm_execution_in_direct_text() {
    let mut shot = valid_shot_value(1);
    shot["motion_prompt"] = json!(
        "[运镜与动作]\n镜头从门口中景缓慢推近，主角站在门口，手指轻触门框并深呼吸，然后把铁片刺入眼睛，镜头推近到面部后稳定停住。\n\n[对白与表演]\n无对白。"
    );
    let raw = response_from_shots(vec![shot]);

    let rows = parse_with_valid_segments(&raw).expect(
        "self-harm execution wording is left to prompt optimization instead of hard validation",
    );

    assert_eq!(rows.len(), 1);
}
